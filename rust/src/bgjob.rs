//! bgjob.h: work in a background process. Rendering a notebook to PDF takes seconds on the
//! Kindle; on the UI thread it froze typing and drawing (and a long one tripped the launcher's
//! watchdog, which killed Grimoire). One child process at a time, niced, works through a small
//! queue; each child loads what it needs itself. Besides the PDF mirror: compacting a notebook's
//! pages (erased ink taken out of the files), installing a template into a notebook, and importing a
//! folder of page images. Since 2.0.0-beta.2 a second child may run beside the first, for one job
//! only: a batch of page previews a view on screen waits on, while the first child is busy (an export,
//! an earlier screenful's dense page), when the processor has three cores or more (extra_slot), and never
//! over pages the other child works on (next_job).

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
    /// id: the notebook being made, path: the PDF's StoragePath; one slice of at most 25 pages or 20 s of a PDF
    /// import (pdfimport.rs). It ends with "more to do" (exit 3) until the last slice, which commits.
    PdfImport = 7,
    /// id: none, path: "" or "recheck:<key>"; the fonts of grimoire/fonts that have no verdict yet, each checked by
    /// the sandboxed worker (userfonts.rs, 2.1.0-beta.2). It ends 0, 3 (some files wait) or 4 (no worker).
    FontCheck = 8,
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
    // (either child: the second only ever runs a previews batch)
    for extra in [false, true] {
        let pid = with(|s| {
            let (pid, j) = if extra { (s.extra_pid, &s.extra) } else { (s.pid, &s.running) };
            if pid > 0 && matches!(j.kind, BgKind::CompactPages | BgKind::GraphLayout | BgKind::PagePreviews) && j.id.as_bytes() == id.as_bytes() {
                pid
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
            with(|s| if extra { s.extra_pid = -1 } else { s.pid = -1 });
        }
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
    let same=|j:&Job|j.kind==kind&&j.id.as_str()==id;
    with(|s|(s.pid>0&&same(&s.running))||(s.extra_pid>0&&same(&s.extra))||s.q.iter().any(same))
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
    with(|s|(s.pid>0&&same(&s.running))||(s.extra_pid>0&&same(&s.extra))||s.q.iter().any(same))
}
// The same, queued only (not started: its child reads what it works on, a notebook's page list, as it starts).
fn bg_job_queued(kind:BgKind,id:&str,path:&str)->bool {
    with(|s|s.q.iter().any(|j|j.kind==kind&&j.id.as_str()==id&&j.dir.as_str()==path))
}
/// The exit status of the job bg_poll_job reported last (a PDF import's: 0 done, 2 done with blank pages).
pub fn bg_last_exit() -> i32 {
    with(|s| s.last_exit)
}
/// How many children run now, 0 to 2 (the runner's bg.children).
pub fn bg_children()->i32 {
    with(|s|(s.pid>0) as i32+(s.extra_pid>0) as i32)
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
    // The first child, then the second: one finished job reported per call (the next call reports the other).
    for extra in [false, true] {
        let pid = with(|s| if extra { s.extra_pid } else { s.pid });
        if pid <= 0 {
            continue;
        }
        let mut st = 0;
        // SAFETY: waitpid on the child this module forked and has not reaped; WNOHANG, and the
        // status goes into our local.
        let r = unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG) };
        if r == pid || r < 0 {
            // (a PDF import slice's exit status says more than done or failed: 2 = done, some pages blank;
            // 3 = more slices to run: its continuation is queued behind whatever ranks higher)
            let code = if r == pid && libc::WIFEXITED(st) { libc::WEXITSTATUS(st) } else { -1 };
            let (kind, notify, id, job) = with(|s| if extra { (s.extra.kind, s.extra.notify, s.extra.id, s.extra) } else { (s.running.kind, s.running_notify, s.running.id, s.running) });
            let is_pdf = kind == BgKind::PdfImport;
            // (a font check's 3: some files were still being copied and wait for the next look)
            let ok = code == 0 || (is_pdf && (code == 2 || code == 3)) || (kind == BgKind::FontCheck && code == 3);
            if let Some(done)=done_id.as_deref_mut(){*done=id;}
            if is_pdf && code == 3 {
                with(|s| if extra { s.extra_pid = -1 } else { s.pid = -1 });
                let (id, dir) = (job.id, job.dir);
                bg_queue(BgKind::PdfImport, Some(id.as_str()), Some(dir.as_str()), job.dpi, false);
                break; // (nothing finished: the notice and the progress record tell how far it is)
            }
            qlog!("bg: %s pid %d %s", K_KIND_NAMES[kind as usize], pid, if ok { "done" } else { "FAILED" });
            if notify || is_pdf || kind == BgKind::GraphLayout || kind == BgKind::GraphLayoutAll {
                result = if ok { 1 } else { -1 };
            }
            if let Some(d) = done_kind.as_mut() {
                **d = kind as i32;
            }
            with(|s| {
                s.last_exit = code;
                if extra { s.extra_pid = -1 } else { s.pid = -1 }
            });
            break;
        }
    }
    start_next();
    result
}
pub fn bg_poll() -> i32 {
    bg_poll_kind(None)
}
pub fn bg_busy() -> bool {
    with(|s| s.pid > 0 || s.extra_pid > 0 || !s.q.is_empty())
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
    let (mut jobs, running) = with(|s| (s.q.clone(), s.pid));
    // A PDF import slice that is running goes on in the detached child too: its next slice is not queued
    // yet, and Grimoire is no longer there to queue it.
    let running_pdf = with(|s| (s.pid > 0 && s.running.kind == BgKind::PdfImport).then_some(s.running));
    if let Some(j) = running_pdf {
        if !jobs.iter().any(|q| q.kind == j.kind && q.id.as_bytes() == j.id.as_bytes()) {
            jobs.insert(0, j);
        }
    }
    if jobs.is_empty() {
        return;
    }
    // SAFETY: fork, as PORTING.md's process model has it: the child below only works through the
    // jobs and ends with _exit.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // SAFETY: setsid detaches the child from the terminal and process group it was forked in.
        unsafe { libc::setsid() };
        // (the import lock goes with it only when it has an import to finish)
        child_setup(jobs.iter().any(|j| is_import(j.kind)));
        // (the detached child runs the jobs left, the biggest margin of them)
        child_caps_apply(jobs.iter().map(|j| child_margin(j.kind)).max().unwrap_or(0));
        // Wait for the running export first (it writes the same folder).
        // SAFETY: kill with signal 0 only checks that the process exists.
        while running > 0 && unsafe { libc::kill(running, 0) } == 0 {
            // SAFETY: usleep has no preconditions.
            unsafe { libc::usleep(200000) };
        }
        for j in &jobs {
            // (a PDF import's slices run back to back: each ends with "more to do" until the last. Each slice makes
            // progress or ends the job; the bound is a last guard, never reached by an import of 2000 pages)
            let mut slices = 0;
            while j.kind == BgKind::PdfImport && slices < 4 * crate::pdfimport::PDF_MAX_PAGES && run_job_guarded(j) == 3 {
                slices += 1;
            }
            if j.kind != BgKind::PdfImport {
                run_job_guarded(j);
            }
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

const K_KIND_NAMES: [&str; 9] = ["export", "compaction", "template", "import", "map layout", "page previews", "library map layout", "PDF import", "font check"];

/// bgjob.c's g_q, g_n, g_pid, g_running_notify and g_running.
struct State {
    q: Vec<Job>,
    pid: i32,
    running_notify: bool,
    /// the job the child is running (valid while pid > 0)
    running: Job,
    /// the second child (2.0.0-beta.2): a previews batch a view waits on, started while the first child
    /// is busy, given the cores (extra_slot); -1 when none
    extra_pid: i32,
    /// the job it runs (valid while extra_pid > 0)
    extra: Job,
    /// a second child could not be forked (the memory short): none is tried again until the first child is
    /// free, which then runs the batch put back in the queue (start_job)
    extra_off: bool,
    /// the exit status of the job bg_poll_job reported last
    last_exit: i32,
}

impl Default for State {
    fn default() -> Self {
        State { q: Vec::new(), pid: -1, running_notify: false, running: Job::default(), extra_pid: -1, extra: Job::default(), extra_off: false, last_exit: 0 }
    }
}

// Whether a second child may run beside the first: three cores or more (std::thread::available_parallelism,
// at run time), one for Grimoire's own work and one for each child. Each child holds about one page canvas
// and one page at a time (a few tens of MB at most), so two stay well within the Kindle's memory. The
// simulator takes GRIMOIRE_SIM_CORES instead, so a scenario can run with one child.
fn extra_slot() -> bool {
    #[cfg(feature = "sim")]
    if let Some(n) = std::env::var("GRIMOIRE_SIM_CORES").ok().and_then(|v| v.parse::<usize>().ok()) {
        return n >= 3;
    }
    static CORES: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CORES.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get())) >= 3
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
    let (t0, drawn0) = (now_ms(), pdf_pages_rendered());
    let rc = if li == 0 { export_mirror_pdf(&nb, j.dpi, j.dir.as_str()) } else { -1 };
    if rc != 0 {
        qlog!("bg: export of %s failed (load %d, export %d)", &j.id, li, rc);
    } else {
        // (the mirror's time and memory, for the device check in docs/TESTING.md: pages drawn against pages
        // found in the cache, and the child's peak resident memory)
        qlog!(
            "bg: export of %s: %d pages, %d drawn, %d ms, VmHWM %d kB",
            &j.id,
            nb.npages(),
            pdf_pages_rendered().wrapping_sub(drawn0) as i32,
            now_ms().wrapping_sub(t0) as i32,
            vm_hwm_kb()
        );
    }
    rc
}

// This process's peak resident memory in kB (Linux's VmHWM line), 0 where it is not known.
fn vm_hwm_kb() -> i32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("VmHWM:").and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<i32>().ok())))
        .unwrap_or(0)
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
        BgKind::PdfImport => crate::pdfimport::bg_pdf_import(j.id.as_str(), j.dir.as_str()),
        BgKind::FontCheck => crate::userfonts::bg_font_check(j.dir.as_str(), j.dpi),
    }
}

/// run_job in a forked child. A panic there (the release build aborts instead: Cargo.toml) must not
/// unwind into the code that forked it, which would run the parent's work a second time in the
/// child: the job has failed, as it has when the C child crashes.
fn run_job_guarded(j: &Job) -> i32 {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_job(j))).unwrap_or(-1)
}

/// The kinds that are imports: their children keep the import lock (the UI holds it while any is queued or
/// running), and no other child does.
fn is_import(kind: BgKind) -> bool {
    matches!(kind, BgKind::PdfImport | BgKind::ImportFolder)
}

// In a forked child: let go of everything inherited from Grimoire (input devices, possibly
// grabbed; the framebuffer; the power-service pipe), and take a name of its own so nobody
// mistakes it for Grimoire. stdout/stderr stay: they are the log. `keep_lock`: an import's child,
// which keeps the import lock's descriptor.
fn child_setup(keep_lock: bool) {
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
    // The import lock's descriptor stays in an import's child (DESIGN.md 3.9): it holds the very lock the UI
    // holds, so the detached child at exit keeps it too. Any other child closes it: a compaction, a mirror or a
    // font check that starts as the last import ends must not hold the lock after the UI lets go of it (H2). It
    // is O_CLOEXEC, so no worker or script inherits it.
    let keep = if keep_lock { crate::pdfimport::lock_fd() } else { -1 };
    for fd in 3..max as i32 {
        if fd == keep {
            continue;
        }
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
        BgKind::ImportFolder | BgKind::PdfImport => 3,
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

// The next job for a free child: the first child takes the most urgent one; while it is busy, the second
// takes the first queued previews batch a view waits on (extra_slot). Both may start at once. A fork that
// fails ends the round (the next poll tries again).
fn start_next() {
    let cores = extra_slot();
    for _ in 0..2 {
        let next = with(|s| next_job(s, cores).map(|(i, extra)| (take(&mut s.q, i), extra)));
        let Some((j, extra)) = next else { return };
        if !start_job(j, extra) {
            return;
        }
    }
}

// A previews batch's page indexes, from its path ("first:last", or "first:last:pid" leaving the open page
// out); None for any other job.
fn previews_range(j: &Job) -> Option<(usize, usize)> {
    if j.kind != BgKind::PagePreviews {
        return None;
    }
    let mut parts = j.dir.as_str().split(':');
    let first = parts.next()?.parse().ok()?;
    let last = parts.next()?.parse().ok()?;
    Some((first, last))
}
// Whether two jobs are previews batches of one notebook over any of the same pages (their index ranges meet).
fn previews_overlap(a: &Job, b: &Job) -> bool {
    a.id.as_bytes() == b.id.as_bytes()
        && matches!((previews_range(a), previews_range(b)), (Some((f1, l1)), Some((f2, l2))) if f1 < l2 && f2 < l1)
}

// Which queued job a free child takes now: its index in the queue, and whether that child is the second.
// The first child takes the most urgent; the second (while the first is busy, given the cores: `cores`)
// the first queued previews batch a view waits on. A previews batch never starts while the other child
// runs one of the same notebook over any of the same pages: both would draw the same pages at once (a dense
// one for half a minute on a Kindle). It waits for that one to end, and then finds its pages made.
fn next_job(s: &mut State, cores: bool) -> Option<(usize, bool)> {
    if s.q.is_empty() {
        return None;
    }
    if s.pid <= 0 {
        s.extra_off = false;
        let mut next: Option<usize> = None;
        for i in 0..s.q.len() {
            if s.extra_pid > 0 && previews_overlap(&s.q[i], &s.extra) {
                continue;
            }
            if next.is_none_or(|n| rank(&s.q[i]) < rank(&s.q[n])) {
                next = Some(i);
            }
        }
        return next.map(|i| (i, false));
    }
    if s.extra_pid > 0 || s.extra_off || !cores {
        return None;
    }
    let i = s.q.iter().position(|j| j.kind == BgKind::PagePreviews && j.notify && !previews_overlap(j, &s.running))?;
    Some((i, true))
}

// A fork that failed for j: a second child's batch goes back to the head of the queue, for the first child
// once it is free (its tiles go on waiting for it), and no second child is tried until then. Any other job
// is skipped (true when it was put back).
fn fork_failed(s: &mut State, j: Job, extra: bool) -> bool {
    if !extra {
        return false;
    }
    s.q.insert(0, j);
    s.extra_off = true;
    true
}

// Forks the child that runs j: the second child when `extra`, else the first. False when the fork failed.
fn start_job(j: Job, extra: bool) -> bool {
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
        if with(|s| fork_failed(s, j, extra)) {
            qlog!("bg: cannot fork a second process; %s of %s waits for the first", K_KIND_NAMES[j.kind as usize], j.id.as_str());
            return false;
        }
        if j.kind != BgKind::MirrorPdf {
            // only the PDF is worth a stall of the UI
            qlog!(
                "bg: cannot fork; %s of %s skipped",
                K_KIND_NAMES[j.kind as usize],
                if j.id.is_empty() { "a folder" } else { j.id.as_str() }
            );
            return false;
        }
        qlog!("bg: cannot fork; exporting %s in the foreground", &j.id);
        run_job(&j);
        return false;
    }
    if pid == 0 {
        // The child only renders and writes files: no display, no input, no atexit handlers.
        child_stops_by_default(&was);
        child_setup(is_import(j.kind));
        child_caps_apply(child_margin(j.kind)); // (oom_score_adj 500 and RLIMIT_AS, childcaps.rs)
        #[cfg(feature = "sim")]
        {
            // Tests: an export as slow as a big notebook's on the Kindle, so the next one queues.
            if let Ok(slow) = std::env::var("GRIMOIRE_BG_DELAY_MS") {
                sleep_ms(atoi(&slow));
            }
        }
        let rc = run_job_guarded(&j);
        // SAFETY: the child ends here without running the parent's exit handlers or flushing its
        // buffers. (A PDF import slice's 2 and 3 mean done with blank pages, and more to do; a font check's 3
        // and 4, some files wait, and no worker can be had.)
        let says_more = (j.kind == BgKind::PdfImport && (rc == 2 || rc == 3)) || (j.kind == BgKind::FontCheck && (rc == 3 || rc == 4));
        unsafe { libc::_exit(if rc == 0 { 0 } else if says_more { rc } else { 1 }) };
    }
    let beside = with(|s| {
        if extra {
            s.extra_pid = pid;
            s.extra = j;
            s.pid
        } else {
            s.pid = pid;
            s.running = j;
            s.running_notify = j.notify;
            -1
        }
    });
    if j.kind == BgKind::MirrorPdf {
        qlog!("bg: exporting %s (pid %d)", &j.id, pid);
    } else if extra {
        qlog!("bg: %s of %s (pid %d, beside pid %d)", K_KIND_NAMES[j.kind as usize], j.id.as_str(), pid, beside);
    } else {
        qlog!(
            "bg: %s of %s (pid %d)",
            K_KIND_NAMES[j.kind as usize],
            if j.id.is_empty() { "a folder" } else { j.id.as_str() },
            pid
        );
    }
    true
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
/// queued, and a view waiting on it puts it before the others (bg_prefer). The same batch queued already
/// (by the idle warm-up, or a view open before) is waited on instead: it reads the notebook's page list as
/// it starts, so it makes these pages as they are now. One running that was not asked for here read the
/// list before, and the pages may have moved since (one deleted, some added): another batch is queued, for
/// what it does not make. That one runs after it (next_job: never two children on the same pages at once)
/// and finds the rest made.
pub fn bg_request_page_previews_but(requests:&mut PagePreviewRequests,nb:&NotebookInfo,first:usize,last:usize,dpi:f32,waited:bool,skip:Option<u32>) {
    let epoch=store_page_previews_epoch(nb.id.as_str());
    if requests.id!=nb.id || requests.epoch!=epoch {
        *requests=PagePreviewRequests{id:nb.id,epoch,..Default::default()};
    }
    let last=last.min(nb.page_ids.len());let first=first.min(last);
    if last-first>64 {return;}
    let range=match skip {Some(pid)=>format!("{first}:{last}:{pid}"),None=>format!("{first}:{last}")};
    if waited {requests.forget_queued(&range);}
    let id=requests.id;
    requests.jobs.retain(|(r,_)|bg_job_pending(BgKind::PagePreviews,id.as_str(),r));
    // Asked for here, and not ended: nothing more to ask.
    if requests.jobs.iter().any(|(r,_)|*r==range) {
        if waited {bg_prefer(BgKind::PagePreviews,nb.id.as_str(),&range);}
        return;
    }
    let queued=bg_job_queued(BgKind::PagePreviews,nb.id.as_str(),&range);
    let pages=&nb.page_ids[first..last];
    let mut missing=Vec::new();
    if missing.try_reserve(pages.len()).is_err() || requests.pages.try_reserve(pages.len()).is_err() || requests.jobs.try_reserve(1).is_err() {
        return;
    }
    for &pid in pages {
        // (a stamp check, not a decode: the view decodes what it shows itself)
        if Some(pid)!=skip && (queued || !requests.pages.contains(&pid)) && store_page_preview_fresh(nb,pid).is_none() {missing.push(pid);}
        if !requests.pages.contains(&pid) {requests.pages.push(pid);}
    }
    if missing.is_empty() {return;}
    // (the others of the notebook made idle work before this one is queued, so it is the one that starts)
    if waited {bg_prefer(BgKind::PagePreviews,nb.id.as_str(),&range);}
    if !queued {bg_queue(BgKind::PagePreviews,Some(nb.id.as_str()),Some(&range),dpi,waited);}
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
// What a decoded preview is of: the page file identity it shows (store_page_source_identity's) and the preview
// file it was read from (store_page_preview_file's; for one made here from the open page's canvas, the preview file
// there was then, if any). Either changed, it is decoded again: a preview written again over the same page file
// (editor.rs retire_kept_before, in place of one 2.0.0 or 2.0.1 made from an old picture) is another picture.
type Of = (String,Option<String>);
// Whether a decoded preview of page pid is still of the page file and the preview file there now.
fn of_now(nb:&NotebookInfo,pid:u32,of:&Of)->bool {
    store_page_source_identity(nb,pid).as_deref()==Some(of.0.as_str()) && store_page_preview_file(nb,pid)==of.1
}
// What a tile was drawn showing, its preview by what it is of (Of): a check later draws again only the tiles
// that would show something else now (PreviewCache::changed).
#[derive(Clone,Debug,PartialEq,Eq)]
enum Shown {
    Preview(Of),
    Loading,
    Number,
}

/// The decoded previews a view shows, for the screenful on it (the Pages view's grid, the link
/// destination picker's). Each keeps the page file identity it shows and the preview file it was read
/// from, so a new preview elsewhere in the notebook keeps the tiles whose pages and previews did not change
/// instead of decoding them all again. It notes what each tile was drawn showing, so that the previews
/// coming in one at a time are drawn one tile at a time.
#[derive(Clone,Debug,Default)]
pub struct PreviewCache {
    notebook: FixedStr<STORE_ID_LEN>,
    generation: u64,
    tiles: Vec<(u32,Of,Canvas)>,
    /// the tiles drawn, in the order they were first drawn (the screenful's), and what each showed
    shown: Vec<(u32,Shown)>,
}
impl PreviewCache {
    /// Before a draw: another notebook empties it; new previews (or a forced check, as a view opens)
    /// keep only the tiles whose page files and preview files are as they were.
    pub fn sync(&mut self,nb:&NotebookInfo,force:bool) {
        if self.notebook!=nb.id {
            self.tiles.clear();self.shown.clear();
            self.notebook=nb.id;self.generation=store_page_previews_generation(nb.id.as_str());return;
        }
        let generation=store_page_previews_generation(nb.id.as_str());
        if generation==self.generation && !force {return;}
        self.generation=generation;
        self.tiles.retain(|(pid,of,_)|of_now(nb,*pid,of));
    }
    // Page pid's tile among the decoded ones, decoded now from the disk cache if it is not yet.
    fn decoded(&mut self,nb:&NotebookInfo,pid:u32)->Option<usize> {
        if let Some(i)=self.tiles.iter().position(|t|t.0==pid) {return Some(i);}
        let mut preview=Canvas::default();
        let (identity,file)=store_load_page_preview_as(nb,pid,&mut preview)?;
        if self.tiles.try_reserve(1).is_err() {return None;}
        self.tiles.push((pid,(identity,Some(file)),preview));
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
    /// How many pixels of page pid's decoded preview are darker than mid-gray (the runner's editor.pages.ink.N),
    /// None while none is decoded.
    pub fn ink(&self,pid:u32)->Option<usize> {
        let (_,_,c)=self.tiles.iter().find(|t|t.0==pid)?;
        Some((0..c.h.max(0) as usize).map(|y|c.px[y*c.stride as usize..][..c.w as usize].iter().filter(|&&v|v<128).count()).sum())
    }
    /// A preview made here (the open page, from its canvas) for the page file at `identity`, beside the preview
    /// file `file` (store_page_preview_file's then: None for none).
    pub fn put(&mut self,pid:u32,identity:String,file:Option<String>,preview:Canvas) {
        self.forget(pid);
        if self.tiles.try_reserve(1).is_ok() {self.tiles.push((pid,(identity,file),preview));}
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

    // A preview written again over the same page file (editor.rs retire_kept_before puts the page drawn in place of an
    // old picture 2.0.0 or 2.0.1 made) is decoded again at the next check, the tile drawn showing it named as changed;
    // so is a tile made from the open page's canvas once a preview is written beside it. Another page's new preview
    // leaves a tile as it is. Before, a tile was kept by its page file alone: the old picture stayed, at every open of
    // the view, until the process ended.
    #[test]
    fn a_preview_written_again_is_decoded_again() {
        let root = std::env::temp_dir().join(format!("grimoire-tiles-again-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        store_set_device(300.0, 600, 800);
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Again", "", PAPER_BLANK, &mut nb), 0);
        assert_eq!(store_add_page(&mut nb, 1, PAPER_BLANK), 0);
        let pids = nb.page_ids.clone();
        let (mut dark, mut white) = (Canvas::default(), Canvas::default());
        assert_eq!(canvas_init(&mut dark, 600, 800), 0);
        assert_eq!(canvas_init(&mut white, 600, 800), 0);
        canvas_fill(&mut dark, Rect::new(0, 0, 600, 800), 40);
        canvas_fill(&mut white, Rect::new(0, 0, 600, 800), 255);
        let mut req = PagePreviewRequests::default();
        let mut cache = PreviewCache::default();
        store_save_page_preview(&nb, pids[0], &dark);
        store_save_page_preview(&nb, pids[1], &white);
        cache.sync(&nb, true);
        assert!(matches!(cache.tile(&nb, pids[0], &mut req, true), Tile::Preview(_)));
        assert!(matches!(cache.tile(&nb, pids[1], &mut req, true), Tile::Preview(_)));
        assert_eq!(cache.ink(pids[0]), Some(300 * 400));
        // Page 1's preview written again, its page file as it was
        let page = store_page_source_identity(&nb, pids[0]);
        store_save_page_preview(&nb, pids[0], &white);
        assert!(page.is_some() && store_page_source_identity(&nb, pids[0]) == page);
        assert_eq!(cache.changed(&nb, &pids, &mut req), vec![pids[0]]);
        assert!(matches!(cache.tile(&nb, pids[0], &mut req, true), Tile::Preview(_)));
        assert_eq!(cache.ink(pids[0]), Some(0));
        assert!(cache.changed(&nb, &pids, &mut req).is_empty());
        // and checked as a view opens
        store_save_page_preview(&nb, pids[0], &dark);
        cache.sync(&nb, true);
        assert!(!cache.has(pids[0]) && cache.has(pids[1]));
        assert!(cache.get(&nb, pids[0]).is_some() && cache.ink(pids[0]) == Some(300 * 400));
        // A tile made from the canvas, its preview on disk older: it goes once a preview is written beside it
        let mut small = Canvas::default();
        assert!(store_page_preview_image(&dark, &mut small));
        cache.put(pids[1], store_page_source_identity(&nb, pids[1]).unwrap(), store_page_preview_file(&nb, pids[1]), small);
        store_save_page_preview(&nb, pids[0], &white);
        cache.sync(&nb, false);
        assert_eq!(cache.ink(pids[1]), Some(300 * 400));
        store_save_page_preview(&nb, pids[1], &white);
        cache.sync(&nb, false);
        assert!(!cache.has(pids[1]));
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

    // Never two children on the same pages: a waited previews batch starts beside the first child only over other
    // pages, or another notebook's (not the same batch, the same range under another open page, nor one that meets
    // it), and the first child, free while the second runs, passes over a batch the second covers for the next one.
    // No child is forked: next_job only says which job would start (the pids are made up).
    #[test]
    fn no_two_children_on_the_same_pages() {
        let mut s = State { pid: 4242, running: queued("same", "0:9:2", false), ..State::default() };
        for path in ["0:9:2", "0:9", "0:24:2", "6:15"] {
            s.q.push(queued("same", path, true));
        }
        assert_eq!(next_job(&mut s, true), None);
        s.q.push(queued("same", "9:18:2", true));
        assert_eq!(next_job(&mut s, true), Some((4, true)));
        s.q.insert(0, queued("other", "0:9", true));
        assert_eq!(next_job(&mut s, true), Some((0, true)));
        // (and none with fewer than three cores)
        assert_eq!(next_job(&mut s, false), None);
        // The second child runs screen 2's batch, and the first is free: it takes the most urgent job not over those
        // pages.
        s.q.remove(0);
        let at = s.q.iter().position(|j| j.dir.as_str() == "9:18:2").unwrap();
        s.extra = s.q.remove(at);
        s.extra_pid = 4343;
        s.pid = -1;
        s.q.insert(0, queued("same", "12:21", true));
        s.q.push(queued("same", "18:27", false));
        assert_eq!(next_job(&mut s, true), Some((1, false)));
        s.q.retain(|j| j.dir.as_str() == "12:21" || j.dir.as_str() == "6:15");
        assert_eq!(next_job(&mut s, true), None);
    }

    // A second child that could not be forked (the memory short): its batch goes back to the head of the queue, no
    // second child is tried while the first is busy, and the first takes the batch once it is free. A first child's
    // job that could not be forked is skipped, as before.
    #[test]
    fn a_failed_second_fork_puts_its_batch_back() {
        let mut s = State { pid: 4242, running: queued("fork", "0:9", false), ..State::default() };
        s.q.push(queued("fork", "18:27", false));
        s.q.push(queued("fork", "9:18", true));
        let (i, extra) = next_job(&mut s, true).unwrap();
        assert!(extra);
        let j = s.q.remove(i);
        assert!(fork_failed(&mut s, j, extra));
        assert_eq!(s.q[0].dir.as_str(), "9:18");
        assert_eq!(next_job(&mut s, true), None);
        s.pid = -1;
        assert_eq!(next_job(&mut s, true), Some((0, false)));
        assert!(!s.extra_off);
        let j = s.q.remove(0);
        assert!(!fork_failed(&mut s, j, false) && s.q.len() == 1);
    }

    // A view asks for a batch that runs already, not asked for here (a view open before, the idle warm-up): it read
    // the page list as it started, and the pages may have moved since, so another is queued, which runs after it. One
    // queued already is waited on instead (it reads the list as it starts), and goes first.
    #[test]
    fn a_running_batch_not_asked_for_here_is_not_waited_on() {
        let root = std::env::temp_dir().join(format!("grimoire-running-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        store_init(root.to_str().unwrap());
        store_set_device(300.0, 600, 800);
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("Running", "", PAPER_BLANK, &mut nb), 0);
        for i in 1..3 {
            assert_eq!(store_add_page(&mut nb, i, PAPER_BLANK), 0);
        }
        let pids = nb.page_ids.clone();
        let id = nb.id.as_str().to_string();
        // (a child that does not exist: nothing here waits on it; the batch queued beside it starts in no slot, as
        // the two cover the same pages)
        with(|s| {
            s.pid = 999_999;
            s.running = queued(&id, "0:3", false);
        });
        let mut req = PagePreviewRequests::default();
        bg_request_page_previews(&mut req, &nb, 0, 3, 300.0, true);
        let waited = |s: &mut State| s.q.iter().filter(|j| j.id.as_str() == id && j.dir.as_str() == "0:3" && j.notify).count();
        assert_eq!(with(waited), 1);
        assert_eq!(req.jobs, vec![("0:3".to_string(), pids.clone())]);
        assert!(pids.iter().all(|&p| req.waiting(p)));
        // Asked again (the next draw): nothing more.
        bg_request_page_previews(&mut req, &nb, 0, 3, 300.0, true);
        assert_eq!(with(|s| s.q.len()), 1);
        // The running one ends: the batch queued here still makes the pages.
        with(|s| s.pid = -1);
        assert!(pids.iter().all(|&p| req.waiting(p)));
        // A view opened afresh over a batch queued (here, the idle warm-up's): waited on, no other is asked for.
        with(|s| s.q.iter_mut().for_each(|j| j.notify = false));
        let mut fresh = PagePreviewRequests::default();
        bg_request_page_previews(&mut fresh, &nb, 0, 3, 300.0, true);
        assert_eq!(with(|s| s.q.iter().map(|j| (j.dir.as_str().to_string(), j.notify)).collect::<Vec<_>>()), vec![("0:3".to_string(), true)]);
        assert_eq!(fresh.jobs, vec![("0:3".to_string(), pids.clone())]);
        with(|s| s.q.clear());
        assert!(!fresh.waiting(pids[0]) && !bg_busy());
        let _ = std::fs::remove_dir_all(&root);
    }
}
