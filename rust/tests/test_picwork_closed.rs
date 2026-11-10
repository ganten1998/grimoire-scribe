// test_picwork_closed.rs: pictures fail closed where the sandbox is required (the owner's ruling of 2026-11-10,
// after the security review's first question: should pictures ever be decoded as root?). With
// GRIMOIRE_SANDBOX=require (what the Kindle build always is) and no worker to be had (a sandbox step fails at the
// handshake, or the exec does not start), no picture is decoded in the writer: a template or photo is not ready
// yet (no `.bad`, and a later scan makes its preview), a picture folder's import counts no try and is taken again
// by every later scan, and the log says why once a process. Where the sandbox is not required the writer still
// decodes them as 2.0 did (test_picwork_fallback.rs). Every picture job runs as the app runs it, in a forked
// child, so this process starts no worker of its own until the last part, which leaves it without workers: its
// own binary.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::pdfimport::*;
use grimoire::picwork::*;
use grimoire::prelude::*;
use testlib::*;

const PW: i32 = 600;
const PH: i32 = 800;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

const HOOKS: [&str; 7] = [
    "GRIMOIRE_PIC_TEST_CRASH",
    "GRIMOIRE_PIC_TEST_HANG",
    "GRIMOIRE_PIC_TEST_TRAP",
    "GRIMOIRE_PIC_WATCHDOG_MS",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_BG_DELAY_MS",
];

/// A worker can be had, and the sandbox is not required.
fn workers_back() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
}

/// The sandbox is required, and every worker's sandbox step fails at its handshake (before a byte is read).
fn no_worker_step() {
    workers_back();
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
}

/// The sandbox is required, and no worker's exec starts.
fn no_worker_exec(root: &str) {
    workers_back();
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    std::env::set_var("GRIMOIRE_WORK_EXE", format!("{root}/no-such-dir/grimoire-headless"));
}

fn setup() -> (String, String) {
    workers_back();
    let storage = format!("{}/picwork-closed-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&storage);
    let root = format!("{storage}/grimoire");
    for d in ["import", "templates", "photos"] {
        std::fs::create_dir_all(format!("{root}/{d}")).unwrap();
    }
    std::fs::create_dir_all(format!("{storage}/documents")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    assert_eq!(store_init(&root), 0);
    store_set_device(300.0, PW, PH);
    ui_reset();
    (root, storage)
}

fn picture(path: &str, shade: u8) {
    let p = BgPicture { px: (0..150 * 200).map(|i| shade.wrapping_add((i % 97) as u8)).collect(), w: 150, h: 200, ch: 1 };
    assert_eq!(bgimage_write_png(path, &p), 0);
}

/// Runs `f` with stderr (the log, also the forked jobs') going to a file; the log's text.
fn with_log<R>(path: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    let r = f();
    let _ = std::io::stderr().flush();
    // SAFETY: dup2 and close on the descriptor saved above.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    let text = std::fs::read_to_string(path).unwrap_or_default();
    eprint!("{text}");
    (r, text)
}

/// Runs the queued background jobs to their end, as app.rs's poll_jobs does.
fn drain() {
    let t0 = std::time::Instant::now();
    loop {
        let mut kind = -1;
        let mut id = FixedStr::<STORE_ID_LEN>::new();
        let r = bg_poll_job(Some(&mut kind), Some(&mut id));
        if kind == BgKind::ImportFolder as i32 {
            let mut pages = 0;
            import_finished(r > 0, &mut pages);
            lock_release_if_idle();
        }
        if !bg_busy() {
            break;
        }
        assert!(t0.elapsed().as_secs() < 120, "the background jobs did not end");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The files of a folder's preview cache with this extension.
fn cached(root: &str, folder: &str, ext: &str) -> usize {
    std::fs::read_dir(format!("{root}/{folder}/.cache"))
        .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(ext)).count())
        .unwrap_or(0)
}

fn notebooks() -> usize {
    let mut all = Vec::new();
    store_list(&mut all);
    all.len()
}

/// The data lines of import/.attempts (none when the file is gone).
fn attempts(root: &str) -> Vec<String> {
    std::fs::read_to_string(format!("{root}/import/.attempts"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(str::to_string)
        .collect()
}

// ---- 1. Templates and photos: not ready yet, never `.bad`, never decoded in the writer ----

fn templates_and_photos_wait() {
    let (root, storage) = setup();
    picture(&format!("{root}/templates/Grid.png"), 10);
    picture(&format!("{root}/templates/Dots.png"), 90);
    picture(&format!("{root}/photos/Beach.png"), 160);
    no_worker_step();
    let (_, text) = with_log(&format!("{storage}/templates.log"), || {
        let mut t: Vec<Template> = Vec::new();
        assert_eq!(template_scan(&mut t, 300.0), 0);
        template_list_free(&mut t);
        drain();
    });
    eprintln!("  templates with no worker: {} preview(s), {} .bad", cached(&root, "templates", ".pgm"), cached(&root, "templates", ".bad"));
    assert!(!text.contains("pictures decode in the writer"), "the writer decoded the templates itself: {text}");
    assert_eq!(cached(&root, "templates", ".pgm"), 0, "a preview was made with no sandboxed worker (decoded in the writer)");
    assert_eq!(cached(&root, "templates", ".bad"), 0, "a template was marked bad for the sandbox's failure");
    assert!(text.contains("templates: 0 preview(s) made, 0 file(s) that are no picture Grimoire can use, 2 left for a later scan"), "{text}");
    // once a process, with its reason (two templates, one job)
    assert_eq!(text.matches("work: pictures wait (").count(), 1, "{text}");
    assert!(text.contains("work: pictures wait (a sandbox step failed)") || text.contains("work: pictures wait (step "), "{text}");
    let (_, text) = with_log(&format!("{storage}/photos.log"), || {
        let mut t: Vec<Template> = Vec::new();
        assert_eq!(photos_scan(&mut t, 300.0), 0);
        template_list_free(&mut t);
        drain();
    });
    assert!(!text.contains("pictures decode in the writer"), "{text}");
    assert_eq!(cached(&root, "photos", ".pgm") + cached(&root, "photos", ".bad"), 0, "a photo was decoded in the writer, or marked bad");
    assert!(text.contains("photos: 0 preview(s) made, 0 file(s) that are no picture Grimoire can use, 1 left for a later scan"), "{text}");
    // with a worker, the next scan makes every preview: nothing was marked
    workers_back();
    for folder in ["templates", "photos"] {
        let mut t: Vec<Template> = Vec::new();
        assert_eq!(scan_folder_named(folder, &mut t), 0);
        template_list_free(&mut t);
        drain();
    }
    assert_eq!((cached(&root, "templates", ".pgm"), cached(&root, "photos", ".pgm")), (2, 1), "the later scan made the previews");
    assert_eq!(cached(&root, "templates", ".bad") + cached(&root, "photos", ".bad"), 0);

    // A template's picture for a notebook (the previews made, then no worker): not made now, nothing marked, and
    // made when the template is picked again with a worker.
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Planner", "", PAPER_BLANK, &mut nb), 0);
    let path = format!("{root}/templates/Grid.png");
    let mut k = PAPER_BLANK;
    let mut why: &'static str = "";
    no_worker_step();
    let (_, text) = with_log(&format!("{storage}/install.log"), || {
        assert_eq!(template_install(&nb, &path, &mut k, &mut why), 0, "{why}");
        drain();
    });
    let mut dir = String::new();
    bgimage_dir(nb.id.as_str(), &mut dir, 700);
    let made = || std::fs::read_dir(&dir).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".png")).count()).unwrap_or(0);
    assert!(!text.contains("pictures decode in the writer"), "{text}");
    assert_eq!(made(), 0, "the notebook's picture was made with no sandboxed worker: {text}");
    assert!(text.contains("could not be made (no sandboxed worker can be had)"), "{text}");
    assert_eq!(cached(&root, "templates", ".bad"), 0);
    workers_back();
    assert_eq!(template_install(&nb, &path, &mut k, &mut why), 0, "{why}");
    drain();
    assert_eq!(made(), 1, "picked again with a worker, the picture is made");
}

fn scan_folder_named(folder: &str, out: &mut Vec<Template>) -> i32 {
    if folder == "photos" {
        photos_scan(out, 300.0)
    } else {
        template_scan(out, 300.0)
    }
}

// ---- 2. A picture folder: no try counted, taken again by every later scan ----

fn a_folder_counts_no_try() {
    let (root, storage) = setup();
    let src = format!("{root}/import/Scans");
    std::fs::create_dir_all(&src).unwrap();
    picture(&format!("{src}/page-1.png"), 30);
    picture(&format!("{src}/page-2.png"), 120);
    // more scans than a folder has tries (3): each one takes it again, as none is counted
    for (run, step) in [true, false, true, false].into_iter().enumerate() {
        if step {
            no_worker_step();
        } else {
            no_worker_exec(&storage);
        }
        let (queued, text) = with_log(&format!("{storage}/import-{run}.log"), || {
            let q = import_scan(300.0);
            drain();
            q
        });
        let att = attempts(&root);
        eprintln!("  scan {}: queued {queued}; notebooks {}; .attempts {att:?}", run + 1, notebooks());
        assert_eq!(queued, 1, "scan {}: the folder was not taken again (a try was counted for the sandbox's failure): {att:?}", run + 1);
        assert_eq!(notebooks(), 0, "scan {}: the folder was imported with no sandboxed worker (decoded in the writer): {text}", run + 1);
        assert!(!text.contains("pictures decode in the writer"), "{text}");
        assert!(text.contains("work: pictures wait ("), "{text}");
        assert!(text.contains("no try is counted; a later scan takes the folder again"), "{text}");
        assert!(att.is_empty(), "scan {}: a try was counted: {att:?}", run + 1);
        assert!(std::path::Path::new(&format!("{src}/page-1.png")).exists(), "the folder stays where it is");
    }
    assert!(!grimoire::bgwork_template::folder_failed(&src));
    // with a worker, the next scan imports it
    workers_back();
    assert_eq!(import_scan(300.0), 1);
    drain();
    assert_eq!(notebooks(), 1, "the later scan imported the folder");
}

// ---- 3. The pool itself, in this process (last: it leaves the process without workers) ----

// A worker that serves its handshake, takes a picture and dies without an answer; the one after it does not
// answer at all. The picture a worker was given is never decoded in the writer (security review 2026-11), and with
// the sandbox required it is not marked bad either: no worker could be had for its second try.
fn retry_worker(dir: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let count = format!("{dir}/started");
    let path = format!("{dir}/worker.sh");
    let script = format!(
        "#!/bin/sh\nPATH=/usr/bin:/bin\nif [ -e '{count}' ]; then exit 0; fi\n: > '{count}'\nprintf 'B %s\\n' '{}' >&4\nprintf 'S 1\\n' >&4\nIFS= read -r line <&5\nsleep 1\nexit 0\n",
        grimoire::sandbox::build_id()
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// What a picture came to, without its pixels.
fn said(o: &PicOut) -> String {
    match o {
        PicOut::Pic { src_w, src_h, .. } => format!("Pic {src_w}x{src_h} (decoded in the writer)"),
        o => format!("{o:?}"),
    }
}

fn the_pool_never_decodes_in_the_writer() {
    let (_root, storage) = setup();
    let path = format!("{storage}/a.png");
    picture(&path, 50);
    let bytes = std::fs::read(&path).unwrap();
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    std::env::set_var("GRIMOIRE_WORK_EXE", retry_worker(&storage));
    let ((handed, later, head), text) = with_log(&format!("{storage}/pool.log"), || {
        let mut pool = PicPool::new();
        let handed = pool.decode(&bytes, false, 300, 400);
        // and every later picture of the process, a step failing now too
        std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
        let later = pool.decode(&bytes, false, 300, 400);
        let head = pool.check(&bytes, bytes.len());
        (handed, later, head)
    });
    workers_back();
    let (handed_s, later_s, head_s) = (said(&handed), said(&later), said(&head));
    eprintln!("  required, no worker: a picture a worker died on {handed_s}; a later one {later_s}; a header {head_s}");
    assert!(matches!(handed, PicOut::NoWorker(_)), "a picture whose second worker could not be had: {handed_s}");
    assert!(matches!(later, PicOut::NoWorker(_)), "a later picture: {later_s} (the writer must not decode it)");
    assert!(matches!(head, PicOut::NoWorker(_)), "a later header: {head_s} (the writer must not read it)");
    assert!(in_fallback(), "no worker can be had in this process");
    assert!(!text.contains("pictures decode in the writer"), "{text}");
    assert_eq!(text.matches("work: pictures wait (").count(), 1, "said once a process: {text}");
}

#[test]
fn test_picwork_closed() {
    run!(templates_and_photos_wait);
    run!(a_folder_counts_no_try);
    run!(the_pool_never_decodes_in_the_writer);
    done();
}
