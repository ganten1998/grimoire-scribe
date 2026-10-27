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
            if s.q[i].kind == BgKind::CompactPages && s.q[i].id.as_bytes() == id.as_bytes() {
                take(&mut s.q, i);
                dropped += 1;
            } else {
                i += 1;
            }
        }
        dropped
    });
    for _ in 0..dropped {
        qlog!("bg: compaction of %s dropped (the notebook opens or goes)", id);
    }
    let pid = with(|s| {
        if s.pid > 0 && s.running.kind == BgKind::CompactPages && s.running.id.as_bytes() == id.as_bytes() {
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
        qlog!("bg: compaction of %s stopped (the notebook opens or goes)", id);
        with(|s| s.pid = -1);
    }
}
/// Reaps a finished child and starts the next job. Returns 1 / -1 when a notify job just
/// succeeded / failed, else 0. Call it often (app_tick). The kind of the job that just finished
/// (notify or not) is in *done_kind when done_kind is not NULL and something finished (else -1).
pub fn bg_poll_kind(done_kind: Option<&mut i32>) -> i32 {
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
            let (kind, notify) = with(|s| (s.running.kind, s.running_notify));
            qlog!("bg: %s pid %d %s", K_KIND_NAMES[kind as usize], pid, if ok { "done" } else { "FAILED" });
            if notify {
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
            if s.q[i].kind == BgKind::CompactPages {
                left.push(take(&mut s.q, i));
            } else {
                i += 1;
            }
        }
        left
    });
    for j in &left {
        qlog!("bg: compaction of %s left for the next run", &j.id);
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

const K_KIND_NAMES: [&str; 4] = ["export", "compaction", "template", "import"];

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

fn run_job(j: &Job) -> i32 {
    match j.kind {
        BgKind::MirrorPdf => run_mirror(j),
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
        BgKind::MirrorPdf => 1,
        BgKind::ImportFolder => 2,
        _ => 3,
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
