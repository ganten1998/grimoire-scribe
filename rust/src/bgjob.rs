//! bgjob.h: work in a background process. Rendering a notebook to PDF takes seconds on the
//! Kindle; on the UI thread it froze typing and drawing (and a long one tripped the launcher's
//! watchdog, which killed Grimoire). One child process at a time, niced, works through a small
//! queue; each child loads what it needs itself. Besides the PDF mirror: compacting a notebook's
//! pages (erased ink taken out of the files), installing a template into a notebook, and importing a
//! folder of page images.

use crate::prelude::*;
use std::cell::RefCell;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BgKind {
    /// id: the notebook; writes grimoire/PDFs/<folder>/<name>.pdf
    #[default]
    MirrorPdf = 0, // BG_MIRROR_PDF
    /// id: the notebook; drops strokes that erasing made invisible (src/bgwork_compact.c)
    CompactPages = 1, // BG_COMPACT_PAGES
    /// id: the notebook, path: the source image; normalizes it into the notebook (src/bgwork_template.c)
    Template = 2, // BG_TEMPLATE
    /// path: a folder of page images; makes a notebook of them (src/bgwork_template.c)
    ImportFolder = 3, // BG_IMPORT_FOLDER
    /// id: notebook, path: graph.ini; layout is computed and saved by the child.
    GraphLayout = 4,
    /// id: notebook, path: "first:last" page indexes; writes the missing small previews.
    PagePreviews = 5,
    /// id: none, path: root/graph-all.ini; the Graph of all notebooks laid out from the library index
    /// (libindex) and saved by the child (2.0.0-beta.2).
    GraphLayoutAll = 6,
}

/// Queues a job (a queued duplicate, same kind, id and path, is merged). notify: report the result
/// through bg_poll (for something the user asked for and waits on). Jobs run one at a time, what the
/// user waits on first (a notify job, a template's picture), then mirror PDFs, imports, compaction;
/// a full queue lets its oldest compaction go first (its pages stay listed).
pub fn bg_queue(kind: BgKind, id: Option<&str>, path: Option<&str>, dpi: f32, notify: bool) {
    let id = id.unwrap_or("");
    let path = path.unwrap_or("");
    let merged = with(|s| {
        for j in s.q.iter_mut() {
            if j.kind == kind && j.id.as_bytes() == id.as_bytes() && j.dir.as_bytes() == path.as_bytes() {
                j.notify |= notify;
                return true;
            }
        }
        false
    });
    if merged {
        start_next();
        return;
    }
    if with(|s| s.q.len()) == BG_QUEUE {
        // The oldest of the last to go first: a compaction (its pages stay listed for the next one)
        // before a mirror, before anything the user waits on.
        let gone = with(|s| {
            let mut drop = 0;
            for i in 1..s.q.len() {
                if rank(&s.q[i]) > rank(&s.q[drop]) {
                    drop = i;
                }
            }
            take(&mut s.q, drop)
        });
        qlog!(
            "bg: queue full, dropping the oldest %s (%s)",
            K_KIND_NAMES[gone.kind as usize],
            if gone.id.is_empty() { "a folder" } else { gone.id.as_str() }
        );
    }
    let j = Job { kind, id: FixedStr::from_str(id), dir: FixedStr::from_str(path), dpi, notify };
    with(|s| s.q.push(j));
    start_next();
}
/// Queues the mirror PDF of notebook `id` (bg_queue with BG_MIRROR_PDF).
pub fn bg_mirror_pdf(id: &str, dpi: f32, mirror_dir: &str, notify: bool) {
    bg_queue(BgKind::MirrorPdf, Some(id), Some(mirror_dir), dpi, notify);
}
/// Before a notebook opens or goes to the trash: its queued compaction is dropped and a running one
/// stopped (its writes are safe to abandon), so the editor and a background rewrite never meet.
/// Returns when it is gone.
pub fn bg_cancel_notebook(id: &str) {
    let dropped = with(|s| {
        let mut dropped = 0;
        let mut i = 0;
        while i < s.q.len() {
            if matches!(s.q[i].kind, BgKind::CompactPages | BgKind::GraphLayout | BgKind::PagePreviews) && s.q[i].id.as_bytes() == id.as_bytes() {
                take(&mut s.q, i);
                dropped += 1;
            } else {
                i += 1;
            }
        }
        dropped
    });
    for _ in 0..dropped {
        qlog!("bg: notebook work for %s dropped (the notebook opens or goes)", id);
    }
    let pid = with(|s| {
        if s.pid > 0 && matches!(s.running.kind, BgKind::CompactPages | BgKind::GraphLayout | BgKind::PagePreviews) && s.running.id.as_bytes() == id.as_bytes() {
            s.pid
        } else {
            -1
        }
    });
    if pid > 0 {
        // SAFETY: kill and waitpid on the child this module forked and has not reaped (pid > 0 in
        // the state); the status goes into our local.
        unsafe {
            libc::kill(pid, libc::SIGTERM);
            let mut st = 0;
            libc::waitpid(pid, &mut st, 0);
        }
        store_cleanup_preview_work(id,pid);
        qlog!("bg: notebook work for %s stopped (the notebook opens or goes)", id);
        with(|s| s.pid = -1);
    }
}
/// A view on screen waits on this very job (kind, notebook id and path), queued: it goes first, as one the
/// user waits on (bg_queue's notify), and the other queued jobs of that kind for the notebook (an earlier
/// screenful's previews, an idle warm-up) wait as idle work behind it, and behind a mirror PDF.
pub fn bg_prefer(kind:BgKind,id:&str,path:&str) {
    with(|s|for j in s.q.iter_mut().filter(|j|j.kind==kind&&j.id.as_str()==id) {j.notify=j.dir.as_str()==path;});
}
/// Running includes an exited child until it is reaped, so a reopened view still receives its result.
pub fn bg_pending(kind:BgKind,id:&str)->bool {
    with(|s|(s.pid>0&&s.running.kind==kind&&s.running.id.as_str()==id)
        ||s.q.iter().any(|j|j.kind==kind&&j.id.as_str()==id))
}
/// Takes this very job (kind, notebook id and path) out of the queue, if it waits there (a running one
/// runs on): true when it did.
pub fn bg_unqueue(kind:BgKind,id:&str,path:&str)->bool {
    with(|s| match s.q.iter().position(|j|j.kind==kind&&j.id.as_str()==id&&j.dir.as_str()==path) {
        Some(i)=>{take(&mut s.q,i);true}
        None=>false,
    })
}
/// The same for this very job: kind, notebook id and path (a previews batch's range).
pub fn bg_job_pending(kind:BgKind,id:&str,path:&str)->bool {
    let same=|j:&Job|j.kind==kind&&j.id.as_str()==id&&j.dir.as_str()==path;
    with(|s|(s.pid>0&&same(&s.running))||s.q.iter().any(same))
}

/// Reaps a finished child and starts the next job. Returns 1 / -1 when a notify job just
/// succeeded / failed, else 0. Call it often (app_tick). The kind of the job that just finished
/// (notify or not) is in *done_kind when done_kind is not NULL and something finished (else -1).
pub fn bg_poll_kind(done_kind: Option<&mut i32>) -> i32 {bg_poll_job(done_kind,None)}

/// The completed job's identity accompanies its kind, before the next queued job starts.
pub fn bg_poll_job(done_kind:Option<&mut i32>,mut done_id:Option<&mut FixedStr<STORE_ID_LEN>>)->i32 {
    if let Some(id)=done_id.as_deref_mut(){id.clear();}
    let mut result = 0;
    let mut done_kind = done_kind;
    if let Some(d) = done_kind.as_mut() {
        **d = -1;
    }
    let pid = with(|s| s.pid);
    if pid > 0 {
        let mut st = 0;
        // SAFETY: waitpid on the child this module forked and has not reaped; WNOHANG, and the
        // status goes into our local.
        let r = unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG) };
        if r == pid || r < 0 {
            let ok = r == pid && libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 0;
            let (kind, notify, id) = with(|s| (s.running.kind, s.running_notify,s.running.id));
            if let Some(done)=done_id.as_deref_mut(){*done=id;}
            qlog!("bg: %s pid %d %s", K_KIND_NAMES[kind as usize], pid, if ok { "done" } else { "FAILED" });
            if notify || kind == BgKind::GraphLayout || kind == BgKind::GraphLayoutAll {
                result = if ok { 1 } else { -1 };
            }
            if let Some(d) = done_kind.as_mut() {
                **d = kind as i32;
            }
            with(|s| s.pid = -1);
        }
    }
    start_next();
    result
}
pub fn bg_poll() -> i32 {
    bg_poll_kind(None)
}
pub fn bg_busy() -> bool {
    with(|s| s.pid > 0 || !s.q.is_empty())
}
/// At exit: hands everything still queued to one detached child, which finishes on its own.
pub fn bg_detach_rest() {
    // Compaction waits for the next run: in a detached child it could still be rewriting pages
    // when the notebook is opened again, out of bg_cancel_notebook's reach.
    let left = with(|s| {
        let mut left = Vec::new();
        let mut i = 0;
        while i < s.q.len() {
            if matches!(s.q[i].kind, BgKind::CompactPages | BgKind::GraphLayout | BgKind::GraphLayoutAll | BgKind::PagePreviews) {
                left.push(take(&mut s.q, i));
            } else {
                i += 1;
            }
        }
        left
    });
    for j in &left {
        qlog!("bg: notebook work for %s left for the next run", &j.id);
    }
    let (jobs, running) = with(|s| (s.q.clone(), s.pid));
    if jobs.is_empty() {
        return;
    }
    // SAFETY: fork, as PORTING.md's process model has it: the child below only works through the
    // jobs and ends with _exit.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // SAFETY: setsid detaches the child from the terminal and process group it was forked in.
        unsafe { libc::setsid() };
        child_setup();
        // Wait for the running export first (it writes the same folder).
        // SAFETY: kill with signal 0 only checks that the process exists.
        while running > 0 && unsafe { libc::kill(running, 0) } == 0 {
            // SAFETY: usleep has no preconditions.
            unsafe { libc::usleep(200000) };
        }
        for j in &jobs {
            run_job_guarded(j);
        }
        // SAFETY: the child ends here without running the parent's exit handlers.
        unsafe { libc::_exit(0) };
    }
    qlog!("bg: %d export(s) left to a detached child (pid %d)", jobs.len() as i32, pid); // (exports and the other jobs)
    with(|s| s.q.clear());
}

/// The work itself, one function per kind other than the PDF mirror (run in the child; 0 = done).
/// (bg_compact_pages is in bgwork_compact.rs, bg_template and bg_import_folder in
/// bgwork_template.rs, as in the C.)
pub use crate::bgwork_compact::bg_compact_pages;
pub use crate::bgwork_template::{bg_import_folder, bg_template};

// ---- bgjob.c ----

const BG_QUEUE: usize = 16;

#[derive(Clone, Copy, Default)]
struct Job {
    kind: BgKind,
    id: FixedStr<STORE_ID_LEN>,
    /// BG_MIRROR_PDF: the mirror folder; the others: their path argument
    dir: FixedStr<600>,
    dpi: f32,
    notify: bool,
}

const K_KIND_NAMES: [&str; 7] = ["export", "compaction", "template", "import", "map layout", "page previews", "library map layout"];

/// bgjob.c's g_q, g_n, g_pid, g_running_notify and g_running.
struct State {
    q: Vec<Job>,
    pid: i32,
    running_notify: bool,
    /// the job the child is running (valid while pid > 0)
    running: Job,
}

impl Default for State {
    fn default() -> Self {
        State { q: Vec::new(), pid: -1, running_notify: false, running: Job::default() }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn run_mirror(j: &Job) -> i32 {
    let mut nb = Box::<NotebookInfo>::default();
    let li = store_load_info(j.id.as_str(), &mut nb);
    let rc = if li == 0 { export_mirror_pdf(&nb, j.dpi, j.dir.as_str()) } else { -1 };
    if rc != 0 {
        qlog!("bg: export of %s failed (load %d, export %d)", &j.id, li, rc);
    }
    rc
}

fn run_graph(j: &Job) -> i32 {
    let mut nb = NotebookInfo::default();
    let mut nav = Nav::default();
    if store_load_info(j.id.as_str(), &mut nb) != 0 || nav_load(j.id.as_str(), &mut nav) != 0 {return -1;}
    let mut graph = GraphModel::default();
    if graph_build(&mut graph,&nb,&nav_for_graph(&nav),0,true) != 0 {return -1;}
    let mut ini = GraphIni::default();
    if graph_ini_read_path(j.dir.as_str(),&mut ini) != 0 {return -1;}
    graph_layout_job(&mut graph,&mut ini,j.dir.as_str())
}

// The Graph of all notebooks: the model built from the library index as the parent had it when it forked
// (it flushed it to graph-index.ini first; read from there if this process never loaded it), the open page
// left out (it is not kept), then graph_all_layout_job.
fn run_graph_all(j: &Job) -> i32 {
    libindex_load();
    let mut graph = GraphModel::default();
    if libindex_graph(&mut graph, None, None) != 0 {return -1;}
    let mut ini = GraphAllIni::default();
    if graph_all_ini_read_path(j.dir.as_str(),&mut ini) != 0 {return -1;}
    graph_all_layout_job(&mut graph,&mut ini,j.dir.as_str())
}

fn run_job(j: &Job) -> i32 {
    match j.kind {
        BgKind::MirrorPdf => run_mirror(j),
        BgKind::GraphLayout => run_graph(j),
        BgKind::GraphLayoutAll => run_graph_all(j),
        BgKind::PagePreviews => store_build_page_previews(j.id.as_str(),j.dir.as_str(),j.dpi),
        BgKind::CompactPages => bg_compact_pages(j.id.as_str(), j.dpi),
        BgKind::Template => bg_template(j.id.as_str(), j.dir.as_str(), j.dpi),
        BgKind::ImportFolder => bg_import_folder(j.dir.as_str(), j.dpi),
    }
}

/// run_job in a forked child. A panic there (the release build aborts instead: Cargo.toml) must not
/// unwind into the code that forked it, which would run the parent's work a second time in the
/// child: the job has failed, as it has when the C child crashes.
fn run_job_guarded(j: &Job) -> i32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_job(j))).unwrap_or(-1)
}

// In a forked child: let go of everything inherited from Grimoire (input devices, possibly
// grabbed; the framebuffer; the power-service pipe), and take a name of its own so nobody
// mistakes it for Grimoire. stdout/stderr stay: they are the log.
fn child_setup() {
    work_pump_set(None); // the pens are Grimoire's to read (common.h), never a child's
    input_forget_signal_pipe(); // (its numbers are closed below, and the jobs' files take them)
    #[cfg(target_os = "linux")]
    // SAFETY: prctl(PR_SET_NAME) reads a NUL-terminated name of at most 16 bytes; the other
    // arguments are unused.
    unsafe {
        libc::prctl(
            libc::PR_SET_NAME,
            c"grimoire-bg".as_ptr() as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
        );
    }
    // SAFETY: sysconf has no preconditions.
    let mut max = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) };
    if !(0..=4096).contains(&max) {
        max = 4096;
    }
    for fd in 3..max as i32 {
        // SAFETY: closing every fd above 2 in the child, as the C does; a failure (not open) is fine.
        unsafe { libc::close(fd) };
    }
    // SAFETY: nice has no preconditions; best effort.
    unsafe { libc::nice(10) };
}

// Which job goes first: something the user waits on (an export they asked for, a template's
// picture), then the mirror PDFs, then imports, and compaction last. A compaction can take minutes
// on the Kindle, and its pages stay listed until it is done, so nothing is lost by its waiting.
fn rank(j: &Job) -> i32 {
    if j.notify || j.kind == BgKind::Template {
        return 0;
    }
    match j.kind {
        BgKind::GraphLayout | BgKind::GraphLayoutAll => 1,
        BgKind::MirrorPdf => 2,
        BgKind::ImportFolder => 3,
        // (an idle warm-up of the Pages view's previews: after anything asked for, with compaction)
        BgKind::PagePreviews => 4,
        _ => 4,
    }
}

fn take(q: &mut Vec<Job>, i: usize) -> Job {
    q.remove(i)
}

// Grimoire's own SIGTERM, SIGINT and SIGHUP handlers only set a flag. A child keeps them until it
// sets its own, and a stop sent in between (bg_cancel_notebook, just after the fork) would be lost:
// Grimoire would wait in waitpid for the whole job. So they are held across the fork, and the child
// puts them back to their defaults before it lets a held one in.
fn stops_set() -> libc::sigset_t {
    // SAFETY: sigset_t is plain data; sigemptyset and sigaddset fill the one we own.
    unsafe {
        let mut s: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut s);
        libc::sigaddset(&mut s, libc::SIGTERM);
        libc::sigaddset(&mut s, libc::SIGINT);
        libc::sigaddset(&mut s, libc::SIGHUP);
        s
    }
}

fn child_stops_by_default(was: &libc::sigset_t) {
    // SAFETY: sigaction is plain data, zeroed then set to the default handler; sigaction and
    // sigprocmask read the structures we own.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = libc::SIG_DFL;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
        libc::sigprocmask(libc::SIG_SETMASK, was, std::ptr::null_mut());
    }
}

/// atoi: the leading number of s (white space and a sign first), 0 for none.
#[cfg(feature = "sim")]
fn atoi(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (9..=13).contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut v: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((b[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

fn start_next() {
    let next = with(|s| {
        if s.pid > 0 || s.q.is_empty() {
            return None;
        }
        let mut next = 0;
        for i in 1..s.q.len() {
            if rank(&s.q[i]) < rank(&s.q[next]) {
                next = i;
            }
        }
        Some(take(&mut s.q, next))
    });
    let Some(j) = next else { return };
    let stops = stops_set();
    // SAFETY: sigset_t is plain data, written by sigprocmask below.
    let mut was: libc::sigset_t = unsafe { std::mem::zeroed() };
    // SAFETY: sigprocmask reads our stops and writes the old mask into was.
    unsafe { libc::sigprocmask(libc::SIG_BLOCK, &stops, &mut was) };
    // SAFETY: fork, as PORTING.md's process model has it: the child below only renders and writes
    // files, and ends with _exit.
    let pid = unsafe { libc::fork() };
    if pid != 0 {
        // SAFETY: puts the old mask back.
        unsafe { libc::sigprocmask(libc::SIG_SETMASK, &was, std::ptr::null_mut()) };
    }
    if pid < 0 {
        if j.kind != BgKind::MirrorPdf {
            // only the PDF is worth a stall of the UI
            qlog!(
                "bg: cannot fork; %s of %s skipped",
                K_KIND_NAMES[j.kind as usize],
                if j.id.is_empty() { "a folder" } else { j.id.as_str() }
            );
            return;
        }
        qlog!("bg: cannot fork; exporting %s in the foreground", &j.id);
        run_job(&j);
        return;
    }
    if pid == 0 {
        // The child only renders and writes files: no display, no input, no atexit handlers.
        child_stops_by_default(&was);
        child_setup();
        #[cfg(feature = "sim")]
        {
            // Tests: an export as slow as a big notebook's on the Kindle, so the next one queues.
            if let Ok(slow) = std::env::var("GRIMOIRE_BG_DELAY_MS") {
                sleep_ms(atoi(&slow));
            }
        }
        let rc = run_job_guarded(&j);
        // SAFETY: the child ends here without running the parent's exit handlers or flushing its
        // buffers.
        unsafe { libc::_exit(if rc == 0 { 0 } else { 1 }) };
    }
    with(|s| {
        s.pid = pid;
        s.running = j;
        s.running_notify = j.notify;
    });
    if j.kind == BgKind::MirrorPdf {
        qlog!("bg: exporting %s (pid %d)", &j.id, pid);
    } else {
        qlog!(
            "bg: %s of %s (pid %d)",
            K_KIND_NAMES[j.kind as usize],
            if j.id.is_empty() { "a folder" } else { j.id.as_str() },
            pid
        );
    }
}

/// One attempt per visible page in an open browser. Missing/read-only pages must not cause
/// completion -> redraw -> another child forever. Explicit cache invalidation allows a retry.
#[derive(Clone,Debug,Default)]
pub struct PagePreviewRequests {
    id: FixedStr<STORE_ID_LEN>,
    epoch: Option<(u64,i64,i64)>,
    pages: Vec<u32>,
    /// The batches asked for and not seen ended: each one's range (its job's path) and the pages it is to
    /// make, whose tiles show the loading mark meanwhile (waiting).
    jobs: Vec<(String,Vec<u32>)>,
}
impl PagePreviewRequests {
    /// Whether page pid's preview is still to come: a batch asked for here is to make it, and is queued or
    /// running. A batch that ended without making it (a page that does not read, one from a newer
    /// Grimoire) leaves its tile with its number alone.
    pub fn waiting(&mut self,pid:u32)->bool {
        let id=self.id;
        self.jobs.retain(|(range,_)|bg_job_pending(BgKind::PagePreviews,id.as_str(),range));
        self.jobs.iter().any(|(_,pages)|pages.contains(&pid))
    }
    // The view turned to another screenful (`keep`): the batches asked for here for the others that are still
    // queued are taken out of the queue (no work for screens passed by), and their pages asked for again when
    // they show again. A running one runs on.
    fn forget_queued(&mut self,keep:&str) {
        let id=self.id;
        let mut i=0;
        while i<self.jobs.len() {
            if self.jobs[i].0!=keep && bg_unqueue(BgKind::PagePreviews,id.as_str(),&self.jobs[i].0) {
                let (_,pages)=self.jobs.remove(i);
                self.pages.retain(|p|!pages.contains(p));
            } else {
                i+=1;
            }
        }
    }
}
/// `waited`: a view on screen shows these pages (else an idle warm-up, which runs after anything asked
/// for and never counts as one the user waits on).
pub fn bg_request_page_previews(requests:&mut PagePreviewRequests,nb:&NotebookInfo,first:usize,last:usize,dpi:f32,waited:bool) {
    bg_request_page_previews_but(requests,nb,first,last,dpi,waited,None)
}
/// The same, leaving out page `skip` (the page open in the editor, whose preview comes from its canvas).
/// A batch is asked for even while another runs (an earlier screenful's, with a dense page in it): it is
/// queued, and a view waiting on it puts it before the others (bg_prefer).
pub fn bg_request_page_previews_but(requests:&mut PagePreviewRequests,nb:&NotebookInfo,first:usize,last:usize,dpi:f32,waited:bool,skip:Option<u32>) {
    let epoch=store_page_previews_epoch(nb.id.as_str());
    if requests.id!=nb.id || requests.epoch!=epoch {
        *requests=PagePreviewRequests{id:nb.id,epoch,..Default::default()};
    }
    let last=last.min(nb.page_ids.len());let first=first.min(last);
    if last-first>64 {return;}
    let range=match skip {Some(pid)=>format!("{first}:{last}:{pid}"),None=>format!("{first}:{last}")};
    if waited {requests.forget_queued(&range);}
    // This very batch queued or running already (asked for here, or by the idle warm-up before the view
    // opened): waited on now, it goes first.
    let pending=bg_job_pending(BgKind::PagePreviews,nb.id.as_str(),&range);
    if pending && waited {bg_prefer(BgKind::PagePreviews,nb.id.as_str(),&range);}
    if pending && requests.jobs.iter().any(|(r,_)|*r==range) {return;}
    let pages=&nb.page_ids[first..last];
    let mut missing=Vec::new();
    if missing.try_reserve(pages.len()).is_err() || requests.pages.try_reserve(pages.len()).is_err() || requests.jobs.try_reserve(1).is_err() {
        return;
    }
    for &pid in pages {
        // (a stamp check, not a decode: the view decodes what it shows itself)
        if Some(pid)!=skip && (pending || !requests.pages.contains(&pid)) && store_page_preview_fresh(nb,pid).is_none() {missing.push(pid);}
        if !requests.pages.contains(&pid) {requests.pages.push(pid);}
    }
    if missing.is_empty() {return;}
    if !pending {
        bg_queue(BgKind::PagePreviews,Some(nb.id.as_str()),Some(&range),dpi,waited);
        if waited {bg_prefer(BgKind::PagePreviews,nb.id.as_str(),&range);}
    }
    requests.jobs.push((range,missing));
}
/// Page pid was saved: its preview may be made once more.
pub fn bg_preview_retry(requests:&mut PagePreviewRequests,pid:u32) {
    requests.pages.retain(|&p|p!=pid);
}

/// What a page tile shows (PreviewCache::tile): its preview, the loading mark while its preview is being
/// made, or its number alone (no preview to come).
pub enum Tile<'a> {
    Preview(&'a Canvas),
    Loading,
    Number,
}
// What a tile was drawn showing, its preview by the page file identity it was made at: a check later
// draws again only the tiles that would show something else now (PreviewCache::changed).
#[derive(Clone,Debug,PartialEq,Eq)]
enum Shown {
    Preview(String),
    Loading,
    Number,
}

/// The decoded previews a view shows, for the screenful on it (the Pages view's grid, the link
/// destination picker's). Each keeps the page file identity it shows, so a new preview elsewhere in the
/// notebook keeps the tiles whose pages did not change instead of decoding them all again. It notes what
/// each tile was drawn showing, so that the previews coming in one at a time are drawn one tile at a time.
#[derive(Clone,Debug,Default)]
pub struct PreviewCache {
    notebook: FixedStr<STORE_ID_LEN>,
    generation: u64,
    tiles: Vec<(u32,String,Canvas)>,
    /// the tiles drawn, in the order they were first drawn (the screenful's), and what each showed
    shown: Vec<(u32,Shown)>,
}
impl PreviewCache {
    /// Before a draw: another notebook empties it; new previews (or a forced check, as a view opens)
    /// keep only the tiles whose page files are as they were.
    pub fn sync(&mut self,nb:&NotebookInfo,force:bool) {
        if self.notebook!=nb.id {
            self.tiles.clear();self.shown.clear();
            self.notebook=nb.id;self.generation=store_page_previews_generation(nb.id.as_str());return;
        }
        let generation=store_page_previews_generation(nb.id.as_str());
        if generation==self.generation && !force {return;}
        self.generation=generation;
        self.tiles.retain(|(pid,identity,_)|store_page_source_identity(nb,*pid).as_deref()==Some(identity.as_str()));
    }
    // Page pid's tile among the decoded ones, decoded now from the disk cache if it is not yet.
    fn decoded(&mut self,nb:&NotebookInfo,pid:u32)->Option<usize> {
        if let Some(i)=self.tiles.iter().position(|t|t.0==pid) {return Some(i);}
        let mut preview=Canvas::default();
        let identity=store_load_page_preview_as(nb,pid,&mut preview)?;
        if self.tiles.try_reserve(1).is_err() {return None;}
        self.tiles.push((pid,identity,preview));
        Some(self.tiles.len()-1)
    }
    /// Page pid's preview: decoded once, from the disk cache, while it stays on screen.
    pub fn get(&mut self,nb:&NotebookInfo,pid:u32)->Option<&Canvas> {
        let i=self.decoded(nb,pid)?;
        Some(&self.tiles[i].2)
    }
    // What page pid's tile would show now.
    fn state(&mut self,nb:&NotebookInfo,pid:u32,req:&mut PagePreviewRequests)->(Shown,Option<usize>) {
        match self.decoded(nb,pid) {
            Some(i)=>(Shown::Preview(self.tiles[i].1.clone()),Some(i)),
            None if req.waiting(pid)=>(Shown::Loading,None),
            None=>(Shown::Number,None),
        }
    }
    /// Page pid's tile as it is to be drawn now: its preview; else, while a batch asked for in `req` is to
    /// make it, the loading mark; else its number alone. `noted`: the tile is drawn whole now (not left
    /// out by a clip), and what it shows is noted for changed().
    pub fn tile(&mut self,nb:&NotebookInfo,pid:u32,req:&mut PagePreviewRequests,noted:bool)->Tile<'_> {
        let (show,at)=self.state(nb,pid,req);
        if noted {
            match self.shown.iter_mut().find(|s|s.0==pid) {
                Some(s)=>s.1=show.clone(),
                None=>if self.shown.try_reserve(1).is_ok() {self.shown.push((pid,show.clone()));},
            }
        }
        match (show,at) {
            (Shown::Preview(_),Some(i))=>Tile::Preview(&self.tiles[i].2),
            (Shown::Loading,_)=>Tile::Loading,
            _=>Tile::Number,
        }
    }
    /// The pages of `pids` (the screenful on show) whose tiles would be drawn otherwise now than they last
    /// were: a preview that came (decoded here), a page whose file changed, a loading mark whose batch ended
    /// without its preview. The tiles not drawn yet are left to their first draw.
    pub fn changed(&mut self,nb:&NotebookInfo,pids:&[u32],req:&mut PagePreviewRequests)->Vec<u32> {
        self.sync(nb,false);
        let mut out=Vec::new();
        for &pid in pids {
            let Some(was)=self.shown.iter().find(|s|s.0==pid).map(|s|s.1.clone()) else{continue};
            if self.state(nb,pid,req).0!=was && out.try_reserve(1).is_ok() {out.push(pid);}
        }
        out
    }
    /// What the tiles drawn show, one letter each in the order they were drawn: `p` a preview, `l` the
    /// loading mark, `n` the number alone (the runner's editor.pages.shows, editor.destination.shows).
    pub fn shows(&self,pids:&[u32])->String {
        pids.iter().filter_map(|p|self.shown.iter().find(|s|s.0==*p)).map(|s|match s.1 {Shown::Preview(_)=>'p',Shown::Loading=>'l',Shown::Number=>'n'}).collect()
    }
    pub fn has(&self,pid:u32)->bool {self.tiles.iter().any(|t|t.0==pid)}
    /// A preview made here (the open page, from its canvas) for the page file at `identity`.
    pub fn put(&mut self,pid:u32,identity:String,preview:Canvas) {
        self.forget(pid);
        if self.tiles.try_reserve(1).is_ok() {self.tiles.push((pid,identity,preview));}
    }
    pub fn forget(&mut self,pid:u32) {self.tiles.retain(|t|t.0!=pid);}
    /// Only the tiles of these pages (the screenful on show) stay.
    pub fn retain(&mut self,pids:&[u32]) {
        self.tiles.retain(|t|pids.contains(&t.0));
        self.shown.retain(|s|pids.contains(&s.0));
    }
    /// The previews generation its tiles were checked at (store_page_previews_generation's then).
    pub fn generation(&self)->u64 {self.generation}
    pub fn len(&self)->usize {self.tiles.len()}
    pub fn is_empty(&self)->bool {self.tiles.is_empty()}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(id: &str, path: &str, notify: bool) -> Job {
        Job { kind: BgKind::PagePreviews, id: FixedStr::from_str(id), dir: FixedStr::from_str(path), dpi: 300.0, notify }
    }

    // A page tile shows the loading mark while a batch asked for is to make its preview, the preview once it
    // is there, and its number alone once the batch ended without one; a check after each change names only
    // the tiles that change (each drawn on its own as its preview lands). No child is started: the batch is
    // put in the queue by hand, and taken out as a child's end would.
    #[test]
    fn tiles_show_loading_until_their_preview_and_change_alone() {
        let root = std::env::temp_dir().join(format!("grimoire-tiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        store_set_device(300.0, 600, 800);
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Tiles", "", PAPER_BLANK, &mut nb), 0);
        for i in 1..3 {
            assert_eq!(store_add_page(&mut nb, i, PAPER_BLANK), 0);
        }
        let pids = nb.page_ids.clone();
        let mut req = PagePreviewRequests::default();
        // (what bg_request_page_previews_but notes as it queues the batch)
        req.id = nb.id;
        req.epoch = store_page_previews_epoch(nb.id.as_str());
        req.pages = pids.clone();
        req.jobs.push(("0:3".to_string(), pids.clone()));
        with(|s| s.q.push(queued(nb.id.as_str(), "0:3", true)));
        let mut cache = PreviewCache::default();
        cache.sync(&nb, true);
        for &pid in &pids {
            assert!(matches!(cache.tile(&nb, pid, &mut req, true), Tile::Loading));
        }
        assert_eq!(cache.shows(&pids), "lll");
        assert!(cache.changed(&nb, &pids, &mut req).is_empty());
        // The second page's preview lands: that tile alone changed, and it shows the preview.
        let mut white = Canvas::default();
        assert_eq!(canvas_init(&mut white, 600, 800), 0);
        canvas_fill(&mut white, Rect::new(0, 0, 600, 800), 255);
        store_save_page_preview(&nb, pids[1], &white);
        assert_eq!(cache.changed(&nb, &pids, &mut req), vec![pids[1]]);
        assert!(matches!(cache.tile(&nb, pids[1], &mut req, true), Tile::Preview(_)));
        assert_eq!(cache.shows(&pids), "lpl");
        assert!(cache.changed(&nb, &pids, &mut req).is_empty());
        // The batch ends without the other two (pages that do not read, say): their loading marks go.
        with(|s| s.q.retain(|j| j.id.as_str() != nb.id.as_str()));
        assert_eq!(cache.changed(&nb, &pids, &mut req), vec![pids[0], pids[2]]);
        for &pid in &[pids[0], pids[2]] {
            assert!(matches!(cache.tile(&nb, pid, &mut req, true), Tile::Number));
        }
        assert_eq!(cache.shows(&pids), "npn");
        // A tile drawn under a clip (not whole) is not noted: it still counts as what the panel shows.
        store_save_page_preview(&nb, pids[0], &white);
        assert!(matches!(cache.tile(&nb, pids[0], &mut req, false), Tile::Preview(_)));
        assert_eq!(cache.shows(&pids), "npn");
        assert_eq!(cache.changed(&nb, &pids, &mut req), vec![pids[0]]);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A view waiting on one batch puts it first: the other queued previews batches of that notebook wait as idle
    // work (notify off), another notebook's are left as they are; and a batch is pending by its very path.
    #[test]
    fn a_waited_batch_goes_first() {
        with(|s| {
            s.q.push(queued("prefer-a", "0:9", true));
            s.q.push(queued("prefer-a", "9:18", false));
            s.q.push(queued("prefer-b", "0:9", true));
        });
        bg_prefer(BgKind::PagePreviews, "prefer-a", "9:18");
        let notify: Vec<(String, String, bool)> =
            with(|s| s.q.iter().map(|j| (j.id.as_str().to_string(), j.dir.as_str().to_string(), j.notify)).collect());
        assert_eq!(notify, vec![("prefer-a".into(), "0:9".into(), false), ("prefer-a".into(), "9:18".into(), true), ("prefer-b".into(), "0:9".into(), true)]);
        assert!(bg_job_pending(BgKind::PagePreviews, "prefer-a", "9:18") && !bg_job_pending(BgKind::PagePreviews, "prefer-a", "18:27"));
        assert!(bg_pending(BgKind::PagePreviews, "prefer-b") && !bg_pending(BgKind::PagePreviews, "prefer-c"));
        with(|s| s.q.retain(|j| !j.id.as_str().starts_with("prefer-")));
        assert!(!bg_busy());
    }

    // The view turned away from a screenful whose batch still waits in the queue: it is taken out, and its pages
    // are asked for again when they show again; the batch of the screenful on show stays.
    #[test]
    fn batches_of_screens_turned_away_from_leave_the_queue() {
        let mut req = PagePreviewRequests::default();
        req.id = FixedStr::from_str("turned");
        req.pages = (1..=18).collect();
        req.jobs.push(("0:9".to_string(), (1..=9).collect()));
        req.jobs.push(("9:18".to_string(), (10..=18).collect()));
        with(|s| {
            s.q.push(queued("turned", "0:9", true));
            s.q.push(queued("turned", "9:18", true));
        });
        req.forget_queued("9:18");
        assert!(!bg_job_pending(BgKind::PagePreviews, "turned", "0:9") && bg_job_pending(BgKind::PagePreviews, "turned", "9:18"));
        assert_eq!(req.jobs.iter().map(|j| j.0.as_str()).collect::<Vec<_>>(), vec!["9:18"]);
        assert_eq!(req.pages, (10..=18).collect::<Vec<u32>>());
        assert!(!req.waiting(3) && req.waiting(12));
        with(|s| s.q.retain(|j| j.id.as_str() != "turned"));
        assert!(!req.waiting(12) && !bg_busy());
    }
}
