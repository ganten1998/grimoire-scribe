//! Two mirror writers of one notebook, and a writer killed midway (2.1.0-beta.2 fix round, H5 and M14). A
//! mirror child outlives Grimoire by design, and Grimoire starts again (every KOReader round trip): a second
//! mirror of the same notebook waits for the first (the notebook's mirror lock), each writes a temporary
//! file of its own (`<name>.pdf.<pid>.tmp`), and the newer state is published last, whole. A writer that is
//! killed leaves its temporary file, which the next mirror of any notebook sweeps. The first writer is held
//! midway by its own work pump: it stops itself as it draws its sixth page (as a slow draw would hold it), until
//! the test lets it go on.

use grimoire::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("pdf-mirror-writers-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn wave(seed: u32, y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut p);
    let mut s = stroke_new(TOOL_PENCIL, &p, seed);
    for i in 0..30 {
        let q = PenSample { x: 100.0 + 20.0 * i as f32, y: y + 6.0 * (i % 5) as f32, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 * 7 };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

fn book(name: &str, pages: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_NARROW_NM, &mut nb), 0);
    for i in 1..pages {
        assert_eq!(store_add_page(&mut nb, i as i32, PAPER_NARROW_NM), 0);
    }
    for i in 0..pages {
        let mut p = page_new(800, 1000, PAPER_NARROW_NM);
        page_add(&mut p, wave(i as u32 + 1, 200.0 + 40.0 * i as f32));
        assert_eq!(store_save_page(&nb, i as i32, &p), 0);
    }
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

fn full(nb: &NotebookInfo) -> Vec<u8> {
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf_to(nb, 300.0, &mut BufSink(&mut b), PdfCacheMode::Off, None), 0);
    b.data
}

/// A notebook's folder in the page-stream cache.
fn cache_dir(nb: &NotebookInfo) -> PathBuf {
    PathBuf::from(pdf_cache_root()).join(nb.id.as_str())
}

fn records_in(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".z")).count()).unwrap_or(0)
}

fn records(nb: &NotebookInfo) -> usize {
    records_in(&cache_dir(nb))
}

/// The temporary files in the mirror's folder.
fn temps(dir: &str) -> Vec<String> {
    std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| n.ends_with(".tmp")).collect()).unwrap_or_default()
}

/// The cache folder of the notebook a held writer mirrors (set before the fork), and whether that writer has
/// held already (in the writer's own memory: each is forked with it false).
static HOLD_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
static HELD: AtomicBool = AtomicBool::new(false);

/// A held writer's work pump (the renderer calls it after every stroke): once its notebook's cache holds five
/// records, which is as its sixth page is drawn, the writer stops itself, until the test sends SIGCONT (or
/// kills it). Once: later calls of the same page still see five records.
fn hold_at_five() {
    if HELD.load(Ordering::Relaxed) {
        return;
    }
    let five = HOLD_DIR.lock().unwrap_or_else(|e| e.into_inner()).as_deref().is_some_and(|d| records_in(d) == 5);
    if five {
        HELD.store(true, Ordering::Relaxed);
        // SAFETY: raise stops this process, the forked writer; it goes on at SIGCONT.
        unsafe { libc::raise(libc::SIGSTOP) };
    }
}

/// The children a part forked: killed and reaped when it ends without having reaped them (a failed assertion
/// would otherwise leave a writer stopped, holding the test's output open).
struct Children(Vec<i32>);
impl Drop for Children {
    fn drop(&mut self) {
        for &pid in &self.0 {
            // SAFETY: kill and waitpid on a child of this process; one already reaped makes both fail harmlessly.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
                let mut st = 0;
                libc::waitpid(pid, &mut st, 0);
            }
        }
    }
}

/// A mirror of nb in a forked child, as bgjob runs it. Exit status 0: export_mirror_pdf said 0; 7: -1. With
/// `hold`, the child stops itself as it draws its sixth page (hold_at_five).
fn mirror_child(nb: &NotebookInfo, dir: &str, hold: bool) -> i32 {
    if hold {
        *HOLD_DIR.lock().unwrap_or_else(|e| e.into_inner()) = Some(cache_dir(nb));
    }
    // SAFETY: fork; the child only exports and _exits.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        if hold {
            work_pump_set(Some(hold_at_five));
        }
        let rc = export_mirror_pdf(nb, 300.0, dir);
        // SAFETY: _exit ends the child at once.
        unsafe { libc::_exit(if rc == 0 { 0 } else { 7 }) };
    }
    pid
}

fn wait_until(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..600 {
        if f() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}");
}

/// Whether the child has ended (its status), without waiting.
fn ended(pid: i32) -> Option<i32> {
    let mut st = 0;
    // SAFETY: waitpid on a child of this process, WNOHANG.
    (unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG) } == pid).then_some(st)
}

/// How many of the xref's objects are not where it says they are (None: no xref where startxref points).
fn xref_misses(pdf: &[u8]) -> Option<(usize, usize)> {
    let tail = &pdf[pdf.len().saturating_sub(64)..];
    let at = tail.windows(10).rposition(|w| w == b"startxref\n")?;
    let num: String = tail[at + 10..].iter().take_while(|b| b.is_ascii_digit()).map(|&b| b as char).collect();
    let x: usize = num.parse().ok()?;
    let head = pdf.get(x..)?;
    if !head.starts_with(b"xref\n0 ") {
        return None;
    }
    let nl = head[7..].iter().position(|&b| b == b'\n')? + 7;
    let n: usize = std::str::from_utf8(&head[7..nl]).ok()?.parse().ok()?;
    let mut misses = 0;
    for i in 1..n {
        let e = &head[nl + 1 + 20 * i..nl + 1 + 20 * i + 10];
        let off: usize = std::str::from_utf8(e).ok()?.parse().ok()?;
        let want = format!("{i} 0 obj\n");
        if !pdf.get(off..).is_some_and(|d| d.starts_with(want.as_bytes())) {
            misses += 1;
        }
    }
    Some((misses, n - 1))
}

#[test]
fn mirror_writers_never_share_a_file() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, 800, 1000);
    a_second_writer_waits_and_publishes_last();
    a_killed_writers_temporary_file_is_swept();
    let _ = std::fs::remove_dir_all(&r);
}

fn a_second_writer_waits_and_publishes_last() {
    let nb = book("Race", 10);
    let dir = format!("{}/PDFs", r_display());
    let pdf = format!("{dir}/Race.pdf");
    // an earlier good mirror is in place
    assert_eq!(export_mirror_pdf(&nb, 300.0, &dir), 0);
    let earlier = std::fs::read(&pdf).unwrap();
    assert_eq!(xref_misses(&earlier), Some((0, 33)));
    std::fs::remove_dir_all(pdf_cache_root()).unwrap(); // (a new version: every page is drawn again)
    // writer A (the mirror an earlier run of Grimoire left running): held at its sixth page
    let a = mirror_child(&nb, &dir, true);
    let mut kids = Children(vec![a]);
    wait_until("A to reach its sixth page", || records(&nb) == 5 && temps(&dir).len() == 1);
    // writer B (Grimoire started again, the notebook changed and closed): the same notebook, one page fewer
    let mut nb_b = nb.clone();
    nb_b.page_ids.remove(5);
    let want_b = full(&nb_b);
    let b = mirror_child(&nb_b, &dir, false);
    kids.0.push(b);
    std::thread::sleep(std::time::Duration::from_millis(600));
    assert!(ended(b).is_none(), "B waits for A (the notebook's mirror lock)");
    assert!(std::fs::read(&pdf).unwrap() == earlier, "nothing is published meanwhile");
    // A goes on: it writes the rest of its PDF into its own temporary file
    // SAFETY: SIGCONT to the child forked above, which stopped itself.
    assert_eq!(unsafe { libc::kill(a, libc::SIGCONT) }, 0);
    let mut st = 0;
    // SAFETY: waitpid on the children forked above.
    unsafe { libc::waitpid(a, &mut st, 0) };
    assert!(libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 0, "A's mirror is written");
    // SAFETY: as above.
    unsafe { libc::waitpid(b, &mut st, 0) };
    assert!(libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 0, "B's mirror is written after it");
    kids.0.clear();
    let last = std::fs::read(&pdf).unwrap();
    assert!(last == want_b, "the newer state is the one published, last");
    assert_eq!(xref_misses(&last), Some((0, 30)), "and it is whole");
    assert!(temps(&dir).is_empty(), "no temporary file is left: {:?}", temps(&dir));
}

fn r_display() -> String {
    root().display().to_string()
}

fn a_killed_writers_temporary_file_is_swept() {
    let nb = book("Killed", 10);
    let dir = format!("{}/PDFs", r_display());
    let a = mirror_child(&nb, &dir, true);
    let kids = Children(vec![a]);
    wait_until("the writer to reach its sixth page", || records(&nb) == 5 && !temps(&dir).is_empty());
    let left = temps(&dir);
    assert_eq!(left, vec![format!("Killed.pdf.{a}.tmp")], "the writer's temporary file is its own");
    // the cap, the kernel's OOM killer (oom_score_adj 500) or a power cut ends it
    drop(kids);
    // a temporary file of a process still running stays (here: this one's), and an older build's young one
    let mine = format!("{dir}/Busy.pdf.{}.tmp", std::process::id());
    std::fs::write(&mine, b"in progress").unwrap();
    let legacy = format!("{dir}/Old.pdf.tmp");
    std::fs::write(&legacy, b"an older build's").unwrap();
    // another notebook's mirror sweeps the killed writer's
    let other = book("Other", 2);
    assert_eq!(export_mirror_pdf(&other, 300.0, &dir), 0);
    let now = temps(&dir);
    assert!(!now.iter().any(|t| t.starts_with("Killed.pdf.")), "a dead writer's temporary file is swept: {now:?}");
    assert!(std::path::Path::new(&mine).exists() && std::path::Path::new(&legacy).exists(), "a running one's stays, and a young one of an older build: {now:?}");
    // an older build's an hour old goes
    let f = std::fs::OpenOptions::new().write(true).open(&legacy).unwrap();
    f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200)).unwrap();
    drop(f);
    assert_eq!(export_mirror_pdf(&other, 300.0, &dir), 0);
    assert!(!std::path::Path::new(&legacy).exists());
    std::fs::remove_file(&mine).unwrap();
}
