// test_file_previews.rs: Files' previews (2.2, issue #145; src/bgwork_previews.rs), the job as the background child
// runs it, on a storage tree of the host's own. Every preview goes through a real worker, a fresh exec of the headless
// runner (GRIMOIRE_WORK_EXE), as in the app: a PDF's first page, a picture, a font's sample. A file the worker refuses
// is marked and not tried again; one whose worker ends or runs out of time gets a try on record first and the mark at
// the second, by one worker run a job, within the preview's own time; a file changed since it was listed gets nothing
// until it is listed again, and then a preview of its own; with no worker to be had nothing is written (and nothing
// decoded in the writer). The cache: an entry that does not read is made again, the entries of files that are gone
// are swept, the prune, the queue's order. The Linux sandbox's steps need root and run in the CI job
// (test_sandbox_linux.rs, `previews_are_made_in_the_real_sandbox`).
#![cfg(feature = "sim")]
mod fontfix;
mod pdffix;
mod testlib;
use grimoire::bgwork_previews::*;
use grimoire::fsview::*;
use grimoire::prelude::*;
use testlib::*;

const HOOKS: [&str; 18] = [
    "GRIMOIRE_STATE",
    "GRIMOIRE_FONT_GLYPH_US",
    "GRIMOIRE_FONT_TEST_SLOW",
    "GRIMOIRE_FONT_TEST_CRASH",
    "GRIMOIRE_PIC_TEST_HANG",
    "GRIMOIRE_PIC_TEST_CRASH",
    "GRIMOIRE_PIC_WATCHDOG_MS",
    "GRIMOIRE_FONT_TEST_HANG",
    "GRIMOIRE_FONT_WATCHDOG_MS",
    "GRIMOIRE_PDF_WATCHDOG_MS",
    "GRIMOIRE_PDF_TEST_HANG_PAGE",
    "GRIMOIRE_PDF_TEST_CRASH_PAGE",
    "GRIMOIRE_PREVIEW_MS",
    "GRIMOIRE_PREVIEWS_MAX",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_PDF_NO_MAP",
    "GRIMOIRE_BG_DELAY_MS",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    grimoire::bgwork_copy::copy_seams(None, None);
}

fn sp(s: &str) -> StoragePath {
    StoragePath::new(s.as_bytes()).unwrap_or_else(|| panic!("{s}"))
}

fn png(w: i32, h: i32) -> Vec<u8> {
    let dir = format!("{}/fp-png", tl_tmp());
    std::fs::create_dir_all(&dir).unwrap();
    let path = format!("{dir}/gen.png");
    let px: Vec<u8> = (0..w * h).map(|i| if (i % w) < w / 2 { 30 } else { 220 }).collect();
    assert_eq!(bgimage_write_png(&path, &BgPicture { px, w, h, ch: 1 }), 0);
    std::fs::read(&path).unwrap()
}

/// A storage tree made afresh, with Grimoire's folder (its store set up there) and documents/Covers holding a PDF, a
/// picture, a font, and one of each kind that its worker refuses.
fn tree() -> (String, String) {
    let root = format!("{}/fp-storage", tl_tmp());
    let _ = std::fs::remove_dir_all(&root);
    let g = format!("{root}/grimoire");
    std::fs::create_dir_all(format!("{root}/documents/Covers")).unwrap();
    std::fs::create_dir_all(&g).unwrap();
    for (f, body) in [
        ("planner.pdf", pdffix::planner(2)),
        ("photo.png", png(120, 90)),
        ("face.ttf", fontfix::ttf(&fontfix::test_font("Regular", 400))),
        ("junk.pdf", pdffix::junk()),
        ("broken.png", b"\x89PNG\r\n\x1a\nnot a picture at all".to_vec()),
        ("web.ttf", fontfix::woff2()),
        ("notes.txt", b"no preview for this kind".to_vec()),
    ] {
        std::fs::write(format!("{root}/documents/Covers/{f}"), body).unwrap();
    }
    std::env::set_var("GRIMOIRE_STORAGE", &root);
    store_init(&g);
    (root, g)
}

fn listed(root: &str, g: &str, p: &str) -> FsStat {
    FsStorage::open(root, g).unwrap().stat(&sp(p), false).unwrap()
}

/// The job's path for a file as listed: what Files' Covers view queues.
fn arg(s: &FsStat, p: &str) -> String {
    format!("{}.{}.{}.{}:{}", s.dev, s.ino, s.size, s.mtime, p)
}

/// The cache's entries (not its sweep's stamp), by name.
fn cache_names(g: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(format!("{g}/.cache/files"))
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.')).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn dark(c: &Canvas) -> usize {
    (0..c.h as usize).map(|y| c.px[y * c.stride as usize..y * c.stride as usize + c.w as usize].iter().filter(|&&v| v < 128).count()).sum()
}

fn preview_of(p: &str, s: &FsStat) -> Option<Canvas> {
    match preview_cached(&sp(p), s) {
        Cached::Preview(c) => Some(c),
        _ => None,
    }
}

fn made(p: &str, s: &FsStat) -> Option<Made> {
    preview_info(&sp(p), s).map(|i| i.made)
}

fn set_mtime(path: &str, mtime: i64) -> bool {
    let t = std::ffi::CString::new(path).unwrap();
    let times = [libc::timeval { tv_sec: mtime as libc::time_t, tv_usec: 0 }; 2];
    // SAFETY: utimes reads the path and the two times we own.
    unsafe { libc::utimes(t.as_ptr(), times.as_ptr()) == 0 }
}

/// Runs `f` with this process's log (stderr) kept in a file: what it returned, and the log's text.
fn with_log<R>(name: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let path = format!("{}/fp-{name}.log", tl_tmp());
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path.as_str()).unwrap();
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
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    eprint!("{text}");
    (r, text)
}

fn settle() {
    let t0 = grimoire::util::real_now_ms();
    while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 60_000 {
        bg_poll();
        sleep_ms(5);
    }
    assert!(!bg_busy());
}

// A PDF's first page, a picture and a font's sample, each made by its worker: a 300 x 400 gray picture with ink in it,
// in the cache under the file's key with its .info; asked again, it is there and no worker runs.
fn a_pdf_a_picture_and_a_font_are_previewed() {
    clean_env();
    let (root, g) = tree();
    for (name, kind) in [("planner.pdf", "pdf"), ("photo.png", "picture"), ("face.ttf", "font")] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(matches!(preview_cached(&sp(&p), &s), Cached::Nothing));
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        let c = preview_of(&p, &s);
        check!(c.as_ref().is_some_and(|c| (c.w, c.h) == (PREVIEW_W, PREVIEW_H) && dark(c) > 50));
        let key = preview_key(p.as_bytes(), s.size, s.mtime);
        let info = std::fs::read_to_string(preview_file(key, "info")).unwrap_or_default();
        check!(info == format!("grimoire-preview 1\nof {} {}\nkind {kind}\nmade 300 400\npath {p}\n", s.size, s.mtime));
        check!(made(&p, &s) == Some(Made::Preview(300, 400)));
        // asked again (a second visit's job queued before the first ended): it is there, and no worker is needed
        std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    }
    // the picture's left half is dark and its right half light, fitted onto the page: so is its preview
    let s = listed(&root, &g, "documents/Covers/photo.png");
    let c = preview_of("documents/Covers/photo.png", &s).unwrap();
    let at = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
    check!(at(80, 200) < 80 && at(220, 200) > 170);
    check!(cache_names(&g).len() == 6);
    // a file of no kind with a preview, and a request that does not read: nothing
    let s = listed(&root, &g, "documents/Covers/notes.txt");
    check!(bg_file_preview(&arg(&s, "documents/Covers/notes.txt")) == -1);
    check!(bg_file_preview("not a request") == -1);
    check!(cache_names(&g).len() == 6);
}

// A file the worker refuses gets a "no preview" mark at once (its .info, no picture), kept: the next job for it finds
// the mark and runs no worker. A PDF that is no PDF, a PNG that is no picture, a web font.
fn a_refused_file_is_marked_and_not_tried_again() {
    clean_env();
    let (root, g) = tree();
    for name in ["junk.pdf", "broken.png", "web.ttf"] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        check!(matches!(preview_cached(&sp(&p), &s), Cached::NoPreview));
        check!(made(&p, &s) == Some(Made::None(NoPreview::Refused)));
        let key = preview_key(p.as_bytes(), s.size, s.mtime);
        check!(!std::path::Path::new(&preview_file(key, "pgm")).exists());
        std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    }
    check!(cache_names(&g).len() == 3);
}

// A worker that never answers is ended when the preview's own time is up (here 500 ms, with every kind's own
// watchdog at its real 30 or 60 s), by ONE worker run: a picture gets no fresh worker's second try. The first time is
// a try on record, which the Covers view reads as nothing to show; the second, at another visit's job, is the mark;
// and a file that then makes its preview after all takes its record's place.
fn a_file_that_takes_too_long_is_tried_twice_within_the_previews_time() {
    clean_env();
    let (root, g) = tree();
    std::env::set_var("GRIMOIRE_PREVIEW_MS", "500");
    for (name, hook, at) in [("photo.png", "GRIMOIRE_PIC_TEST_HANG", "1"), ("face.ttf", "GRIMOIRE_FONT_TEST_HANG", "0"), ("planner.pdf", "GRIMOIRE_PDF_TEST_HANG_PAGE", "1")] {
        std::env::set_var(hook, at);
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        let t = std::time::Instant::now();
        let (rc, log) = with_log("slow-first", || bg_file_preview(&arg(&s, &p)));
        // (ended by the preview's time, far under its kind's watchdog)
        check!(rc == 0 && t.elapsed().as_millis() >= 400 && t.elapsed().as_secs() < 20);
        check!(made(&p, &s) == Some(Made::Once(NoPreview::Slow)));
        check!(matches!(preview_cached(&sp(&p), &s), Cached::Nothing));
        check!(log.contains("gave no preview (slow)") && log.contains("one more try at another visit"));
        // one worker run: no second worker for a picture
        check!(!log.contains("a fresh worker tries it again"));
        let (rc, log) = with_log("slow-second", || bg_file_preview(&arg(&s, &p)));
        check!(rc == 0 && made(&p, &s) == Some(Made::None(NoPreview::Slow)));
        check!(log.contains("has no preview (slow), marked"));
        check!(matches!(preview_cached(&sp(&p), &s), Cached::NoPreview));
        std::env::remove_var(hook);
    }
    // a try on record, and then a worker that makes it: the preview
    std::env::set_var("GRIMOIRE_PIC_TEST_HANG", "1");
    std::fs::write(format!("{root}/documents/Covers/other.png"), png(60, 40)).unwrap();
    let p = "documents/Covers/other.png";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0 && made(p, &s) == Some(Made::Once(NoPreview::Slow)));
    std::env::remove_var("GRIMOIRE_PIC_TEST_HANG");
    check!(bg_file_preview(&arg(&s, p)) == 0 && made(p, &s) == Some(Made::Preview(300, 400)));
    check!(preview_of(p, &s).is_some());
    clean_env();
}

// A worker that ends on a file (here it crashes) is a try, as a page's worker that dies is a strike in an import:
// the first death writes no mark, only its record; the second in a row is the mark. A kind's own watchdog still
// holds inside the preview's time.
fn a_worker_that_ends_is_a_try_and_the_second_the_mark() {
    clean_env();
    let (root, g) = tree();
    for (name, hook, at) in [("photo.png", "GRIMOIRE_PIC_TEST_CRASH", "1"), ("face.ttf", "GRIMOIRE_FONT_TEST_CRASH", "0"), ("planner.pdf", "GRIMOIRE_PDF_TEST_CRASH_PAGE", "1")] {
        std::env::set_var(hook, at);
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        check!(made(&p, &s) == Some(Made::Once(NoPreview::Ended)));
        check!(matches!(preview_cached(&sp(&p), &s), Cached::Nothing));
        let key = preview_key(p.as_bytes(), s.size, s.mtime);
        check!(!std::path::Path::new(&preview_file(key, "pgm")).exists());
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        check!(made(&p, &s) == Some(Made::None(NoPreview::Ended)));
        std::env::remove_var(hook);
    }
    // the kind's watchdog, shorter than the preview's time: it ends the worker
    std::fs::write(format!("{root}/documents/Covers/other.png"), png(60, 40)).unwrap();
    std::env::set_var("GRIMOIRE_PIC_TEST_HANG", "1");
    std::env::set_var("GRIMOIRE_PIC_WATCHDOG_MS", "400");
    let p = "documents/Covers/other.png";
    let s = listed(&root, &g, p);
    let t = std::time::Instant::now();
    check!(bg_file_preview(&arg(&s, p)) == 0 && made(p, &s) == Some(Made::Once(NoPreview::Slow)));
    check!(t.elapsed().as_millis() < 5000);
    clean_env();
}

// A file changed since it was listed (another picture written over it): the file listed again has another key, and
// its job makes a preview of its own, of the new picture; the old listing's preview stays (its job finds it there and
// runs no worker) until the cache is tidied. A file that changes between its listing and its job: nothing is written.
fn a_changed_file_makes_a_new_preview() {
    clean_env();
    let (root, g) = tree();
    let p = "documents/Covers/photo.png";
    let old = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&old, p)) == 0);
    check!(cache_names(&g).len() == 2);
    // all light now, and later
    let path = format!("{root}/{p}");
    check!(bgimage_write_png(&path, &BgPicture { px: vec![235u8; 64 * 48], w: 64, h: 48, ch: 1 }) == 0);
    check!(set_mtime(&path, old.mtime + 5));
    let new = listed(&root, &g, p);
    check!(!new.same(&old));
    check!(preview_key(p.as_bytes(), new.size, new.mtime) != preview_key(p.as_bytes(), old.size, old.mtime));
    check!(matches!(preview_cached(&sp(p), &new), Cached::Nothing));
    check!(bg_file_preview(&arg(&new, p)) == 0);
    check!(preview_of(p, &new).is_some_and(|c| dark(&c) == 0));
    check!(cache_names(&g).len() == 4);
    // (the old listing's preview is the old picture's, and only that listing finds it)
    check!(preview_of(p, &old).is_some_and(|c| dark(&c) > 50));
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(bg_file_preview(&arg(&old, p)) == 0);
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    // changed between its listing and its job: nothing written for either
    let f = "documents/Covers/face.ttf";
    let s = listed(&root, &g, f);
    check!(set_mtime(&format!("{root}/{f}"), s.mtime + 7));
    check!(bg_file_preview(&arg(&s, f)) == -1);
    check!(cache_names(&g).len() == 4);
    check!(bg_file_preview(&arg(&listed(&root, &g, f), f)) == 0);
    check!(cache_names(&g).len() == 6);
}

// A .info or picture that does not read is no preview, to the Covers view and to the job alike, and the next job makes
// it again: a picture a byte short, a picture gone (a good .info beside each), a .info of another file (its size, its
// kind, its path: two paths never share an entry), a .info cut short, a link in a .info's place. A cache folder that
// is a link is not written, and its entries are not taken as there.
fn a_cache_entry_that_does_not_read_is_made_again() {
    clean_env();
    let (root, g) = tree();
    let p = "documents/Covers/photo.png";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0);
    let key = preview_key(p.as_bytes(), s.size, s.mtime);
    let (info, pgm) = (preview_file(key, "info"), preview_file(key, "pgm"));
    let good = std::fs::read(&pgm).unwrap();
    let good_info = std::fs::read_to_string(&info).unwrap();
    // the picture a byte short beside a good .info: nothing to show, and the job makes it again
    std::fs::write(&pgm, &good[..good.len() - 1]).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(std::fs::read(&pgm).is_ok_and(|d| d == good) && preview_of(p, &s).is_some());
    // the picture gone: the same
    std::fs::remove_file(&pgm).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(std::fs::read(&pgm).is_ok_and(|d| d == good) && preview_of(p, &s).is_some());
    // a .info of another file: its size, its kind, its picture's size, its path
    for other in [
        good_info.replace(&format!("of {} ", s.size), &format!("of {} ", s.size + 1)),
        good_info.replace("kind picture", "kind font"),
        good_info.replace("made 300 400", "made 300 399"),
        good_info.replace("photo.png", "other.png"),
        good_info[..good_info.len() - 3].to_string(),
    ] {
        std::fs::write(&info, &other).unwrap();
        check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
        check!(bg_file_preview(&arg(&s, p)) == 0);
        check!(std::fs::read_to_string(&info).is_ok_and(|t| t == good_info) && preview_of(p, &s).is_some());
    }
    // a link in the .info's place: not read (never followed), and the job's atomic write takes its place
    std::fs::remove_file(&info).unwrap();
    std::os::unix::fs::symlink(format!("{g}/settings.ini"), &info).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_of(p, &s).is_some() && !std::path::Path::new(&format!("{g}/settings.ini")).exists());
    // the cache folder a link to a folder elsewhere that holds the entry: the job writes nothing and takes nothing there
    let dir = format!("{g}/.cache/files");
    let elsewhere = format!("{root}/elsewhere");
    std::fs::rename(&dir, &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &dir).unwrap();
    let (rc, log) = with_log("linked", || bg_file_preview(&arg(&s, p)));
    check!(rc == -1 && log.contains("nothing made (the cache folder cannot be made)"));
    std::fs::remove_file(&dir).unwrap();
}

// A font's preview is no check of the font (#111): a font in grimoire/fonts, previewed, has no verdict, no entry in
// the fonts' own cache and no mark there: the folder where verdicts are kept stays empty, and grimoire/fonts holds
// the font file alone, so the font still waits for its check and nothing draws text with it. A font whose sample's
// letter is over the check's glyph limit has no preview (a try, then the mark, as taking too long), and no verdict
// either.
fn a_fonts_preview_is_no_verdict() {
    clean_env();
    let (root, g) = tree();
    let state = format!("{}/fp-state", tl_tmp());
    let _ = std::fs::remove_dir_all(&state);
    std::fs::create_dir_all(&state).unwrap();
    std::env::set_var("GRIMOIRE_STATE", &state);
    std::fs::create_dir_all(format!("{g}/fonts")).unwrap();
    let font = fontfix::test_font("Regular", 400);
    std::fs::write(format!("{g}/fonts/Mine.ttf"), fontfix::ttf(&font)).unwrap();
    let p = "grimoire/fonts/Mine.ttf";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_of(p, &s).is_some_and(|c| dark(&c) > 50));
    let files_under = |d: &str| -> Vec<String> {
        fn walk(d: &std::path::Path, out: &mut Vec<String>) {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                if e.path().is_dir() {
                    walk(&e.path(), out);
                } else {
                    out.push(e.path().to_string_lossy().into_owned());
                }
            }
        }
        let mut v = Vec::new();
        walk(std::path::Path::new(d), &mut v);
        v
    };
    // nothing where verdicts are kept, nothing in the fonts' own cache: only the font itself under fonts/
    check!(files_under(&state).is_empty());
    check!(files_under(&format!("{g}/fonts")) == vec![format!("{g}/fonts/Mine.ttf")]);
    // a letter of the sample over the glyph limit: no preview, a try and then the mark, and still no verdict
    std::fs::write(format!("{g}/fonts/Slow.ttf"), fontfix::ttf(&fontfix::test_font("Bold", 700))).unwrap();
    let slow = "grimoire/fonts/Slow.ttf";
    let s2 = listed(&root, &g, slow);
    std::env::set_var("GRIMOIRE_FONT_GLYPH_US", "1000");
    std::env::set_var("GRIMOIRE_FONT_TEST_SLOW", "from:0:5000");
    check!(bg_file_preview(&arg(&s2, slow)) == 0);
    check!(made(slow, &s2) == Some(Made::Once(NoPreview::Slow)));
    check!(bg_file_preview(&arg(&s2, slow)) == 0);
    check!(made(slow, &s2) == Some(Made::None(NoPreview::Slow)));
    check!(files_under(&state).is_empty());
    clean_env();
}

// The previews of files that are gone go with the cache's sweep: once a day (the first job that writes when the
// last sweep is a day old, or there was none), each entry's .info is read and the entry dropped when its file is no
// longer there as it was: deleted, renamed, or written over. Within the day no sweep runs, unless the cache is past a
// limit: then the gone ones go first, before any preview of a file that is there.
fn the_previews_of_files_that_are_gone_are_swept() {
    clean_env();
    let (root, g) = tree();
    let dir = format!("{g}/.cache/files");
    let names = ["planner.pdf", "photo.png", "face.ttf", "junk.pdf"];
    let job = |name: &str| {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        (bg_file_preview(&arg(&s, &p)), format!("{:016x}", preview_key(p.as_bytes(), s.size, s.mtime)))
    };
    let stems: Vec<String> = names.iter().map(|n| job(n)).map(|(rc, stem)| {
        check!(rc == 0);
        stem
    }).collect();
    let has = |stem: &str| cache_names(&g).iter().any(|n| n.starts_with(stem));
    check!(stems.iter().all(|s| has(s)) && cache_names(&g).len() == 7);
    // (the first job that wrote swept, and left its stamp: an empty file)
    let stamp = format!("{dir}/.swept");
    check!(std::fs::read(&stamp).is_ok_and(|b| b.is_empty()));
    // the PDF deleted, the photo written over, the font renamed: within the day, nothing is swept
    std::fs::remove_file(format!("{root}/documents/Covers/planner.pdf")).unwrap();
    std::fs::write(format!("{root}/documents/Covers/photo.png"), png(50, 50)).unwrap();
    std::fs::rename(format!("{root}/documents/Covers/face.ttf"), format!("{root}/documents/Covers/face2.ttf")).unwrap();
    let (rc, web) = job("web.ttf");
    check!(rc == 0 && stems.iter().all(|s| has(s)) && has(&web));
    // a day later: the next job that writes sweeps them, and keeps what is of a file that is there (the mark too)
    check!(set_mtime(&stamp, wall_time() - 86_401));
    let ((rc, broken), log) = with_log("sweep", || job("broken.png"));
    check!(rc == 0 && log.contains("3 preview(s) of files that are gone or changed dropped from the cache"));
    check!(!has(&stems[0]) && !has(&stems[1]) && !has(&stems[2]));
    check!(has(&stems[3]) && has(&web) && has(&broken));
    check!(std::fs::metadata(&stamp).is_ok_and(|m| m.modified().is_ok_and(|t| t.elapsed().is_ok_and(|e| e.as_secs() < 60))));
    // a sweep that had to stop left the stem it stopped after, and the next job that writes goes on after it at once,
    // not a day later: from the last stem there is nothing left to check (the mark of junk.pdf, deleted now, stays),
    // and from the first every entry is checked (it goes). Either way the sweep went through: the stamp is empty.
    std::fs::remove_file(format!("{root}/documents/Covers/junk.pdf")).unwrap();
    for (i, (from, stays)) in [("ffffffffffffffff", true), ("0000000000000000", false)].into_iter().enumerate() {
        std::fs::write(&stamp, from).unwrap();
        let name = format!("new{i}.png");
        std::fs::write(format!("{root}/documents/Covers/{name}"), png(30 + i as i32, 30)).unwrap();
        let (rc, _) = job(&name);
        check!(rc == 0 && std::fs::read(&stamp).is_ok_and(|b| b.is_empty()));
        check!(has(&stems[3]) == stays);
    }
    // past a limit: the gone ones go first, and then nothing of a file that is there has to go
    clean_env();
    let (root, g) = tree();
    let made_stem = |name: &str| {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        format!("{:016x}", preview_key(p.as_bytes(), s.size, s.mtime))
    };
    let (pdf, photo) = (made_stem("planner.pdf"), made_stem("photo.png"));
    std::fs::remove_file(format!("{root}/documents/Covers/planner.pdf")).unwrap();
    // at most 4 files: the third preview makes 6; the gone PDF's two go, and the photo's (the oldest left) stay
    std::env::set_var("GRIMOIRE_PREVIEWS_MAX", "4,100000000");
    let font = made_stem("face.ttf");
    let left = cache_names(&g);
    check!(left.len() == 4 && !left.iter().any(|n| n.starts_with(&pdf)));
    check!(left.iter().any(|n| n.starts_with(&photo)) && left.iter().any(|n| n.starts_with(&font)));
    clean_env();
}

// The prune, through the job: past its limits (made small here) the oldest previews go, each with all of its files,
// down to three quarters; the one just made stays. Entries that are no plain file are not counted and not touched.
// Deleting the cache loses nothing: the next job makes it again.
fn the_cache_is_pruned_oldest_first() {
    clean_env();
    let (root, g) = tree();
    let files = ["planner.pdf", "photo.png", "face.ttf"];
    for name in &files[..2] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == 0);
    }
    // the two made: an hour and two hours ago
    let dir = format!("{g}/.cache/files");
    for (i, name) in files[..2].iter().enumerate() {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        for ext in ["pgm", "info"] {
            check!(set_mtime(&preview_file(preview_key(p.as_bytes(), s.size, s.mtime), ext), wall_time() - 7200 + 3600 * i as i64));
        }
    }
    check!(cache_names(&g).len() == 4);
    // six folders named as entries are: no plain files, so not counted. With the third preview the cache holds 6
    // files, within a limit of 6, and nothing more is read of it: no prune, and no sweep before its day (the entry
    // of the PDF, deleted now, stays)
    for i in 0..6 {
        std::fs::create_dir(format!("{dir}/{:016x}.pgm", 0xabc0 + i)).unwrap();
    }
    std::fs::remove_file(format!("{root}/documents/Covers/planner.pdf")).unwrap();
    std::env::set_var("GRIMOIRE_PREVIEWS_MAX", "6,100000000");
    let p = "documents/Covers/face.ttf";
    let s = listed(&root, &g, p);
    let (rc, log) = with_log("prune-none", || bg_file_preview(&arg(&s, p)));
    check!(rc == 0 && !log.contains("the cache pruned") && !log.contains("dropped from the cache") && cache_names(&g).len() == 6 + 6);
    // at most 4 files, with two more made: the sweep first (the deleted PDF's entry), then the prune leaves 3 files
    // at most, the newest first
    for ext in ["pgm", "info"] {
        check!(set_mtime(&preview_file(preview_key(p.as_bytes(), s.size, s.mtime), ext), wall_time() - 1800));
    }
    std::env::set_var("GRIMOIRE_PREVIEWS_MAX", "4,100000000");
    std::fs::write(format!("{root}/documents/Covers/more.png"), png(20, 20)).unwrap();
    let more = "documents/Covers/more.png";
    let ms = listed(&root, &g, more);
    let (rc, log) = with_log("prune", || bg_file_preview(&arg(&ms, more)));
    check!(rc == 0 && log.contains("1 preview(s) of files that are gone or changed dropped") && log.contains("the cache pruned: 4 file(s)"));
    let left: Vec<String> = cache_names(&g).into_iter().filter(|n| !n.starts_with("000000000000abc")).collect();
    check!(left.len() == 2 && left.iter().all(|n| n.starts_with(&format!("{:016x}", preview_key(more.as_bytes(), ms.size, ms.mtime)))));
    check!(cache_names(&g).len() == 2 + 6);
    clean_env();
    // the folder deleted: nothing lost, the next job makes the folder and the preview again
    std::fs::remove_dir_all(&dir).unwrap();
    check!(preview_of(p, &s).is_none());
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_of(p, &s).is_some());
}

// Under the storage's free floor no preview is made (the room is the notebooks'): nothing is written, no mark.
fn a_nearly_full_storage_makes_no_preview() {
    clean_env();
    let (root, g) = tree();
    let p = "documents/Covers/photo.png";
    let s = listed(&root, &g, p);
    grimoire::bgwork_copy::copy_seams(Some(FS_FREE_FLOOR + 1000), None);
    let (rc, log) = with_log("full", || bg_file_preview(&arg(&s, p)));
    check!(rc == -1 && log.contains("nothing made (the storage is nearly full)") && cache_names(&g).is_empty());
    grimoire::bgwork_copy::copy_seams(Some(FS_FREE_FLOOR + (1 << 20)), None);
    check!(bg_file_preview(&arg(&s, p)) == 0 && preview_of(p, &s).is_some());
    clean_env();
}

// Through the job queue, as Files asks: the request queues one job a file (a duplicate merged), a background child
// makes the preview, and the cache has it once the child is reaped.
fn the_request_queues_a_job_a_child_runs() {
    clean_env();
    let (root, g) = tree();
    let p = "documents/Covers/planner.pdf";
    let s = listed(&root, &g, p);
    let j = preview_request(&sp(p), &s);
    check!(j.as_deref() == Some(arg(&s, p).as_str()));
    check!(preview_request(&sp(p), &s) == j);
    check!(preview_request(&sp("documents/Covers/notes.txt"), &listed(&root, &g, "documents/Covers/notes.txt")).is_none());
    settle();
    check!(!bg_job_pending(BgKind::FilePreviews, "", j.as_deref().unwrap_or("")));
    check!(preview_of(p, &s).is_some());
}

// The queue's order: a preview is never ahead of real work. With one preview running and two more queued behind it,
// a font check and a template's picture queued after them start before the two (each as its own child, in the order
// the log names them); the previews go last, in the order they were asked for.
fn previews_wait_behind_the_other_jobs() {
    clean_env();
    let (root, g) = tree();
    std::fs::create_dir_all(format!("{g}/fonts")).unwrap();
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "300");
    let ((), log) = with_log("order", || {
        for name in ["planner.pdf", "photo.png", "face.ttf"] {
            let p = format!("documents/Covers/{name}");
            check!(preview_request(&sp(&p), &listed(&root, &g, &p)).is_some());
        }
        // (work that is not asked for by name either, and still goes first: a font check ranks above a preview)
        bg_queue(BgKind::FontCheck, None, Some(""), 300.0, false);
        // (and something the user waits on: a template's picture)
        bg_queue(BgKind::Template, Some("no-such-notebook"), Some("/nonexistent.png"), 300.0, true);
        settle();
    });
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    let started: Vec<&str> = log
        .lines()
        .filter(|l| l.contains(" (pid ") && l.contains("bg: "))
        .map(|l| if l.contains("file previews of") { "preview" } else if l.contains("font check of") { "fonts" } else if l.contains("template of") { "template" } else { "other" })
        .collect();
    check!(started == ["preview", "template", "fonts", "preview", "preview"]);
    for name in ["planner.pdf", "photo.png", "face.ttf"] {
        let p = format!("documents/Covers/{name}");
        check!(preview_of(&p, &listed(&root, &g, &p)).is_some());
    }
}

// No worker to be had (none starts, or a sandbox step fails): nothing is written, no mark and no try, and nothing is
// decoded in the writer; the next visit asks again. In a session whose sandbox step failed nothing more is asked for.
// (Last: it leaves this process without picture workers.)
fn no_worker_writes_nothing() {
    clean_env();
    let (root, g) = tree();
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    for name in ["planner.pdf", "face.ttf", "photo.png"] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == -1);
        check!(matches!(preview_cached(&sp(&p), &s), Cached::Nothing));
    }
    check!(cache_names(&g).is_empty());
    clean_env();
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    for name in ["planner.pdf", "photo.png", "face.ttf"] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        let (rc, log) = with_log("nosandbox", || bg_file_preview(&arg(&s, &p)));
        check!(rc == -1 && log.contains("nothing made (no sandboxed worker can be had)"));
        check!(matches!(preview_cached(&sp(&p), &s), Cached::Nothing));
    }
    check!(cache_names(&g).is_empty());
    // (a step failed in this process: the session is broken, and Files queues no preview in it)
    check!(grimoire::sandbox::session_broken());
    let p = "documents/Covers/photo.png";
    check!(preview_request(&sp(p), &listed(&root, &g, p)).is_none() && !bg_busy());
    clean_env();
}

#[test]
fn test_file_previews() {
    run!(a_pdf_a_picture_and_a_font_are_previewed);
    run!(a_refused_file_is_marked_and_not_tried_again);
    run!(a_file_that_takes_too_long_is_tried_twice_within_the_previews_time);
    run!(a_worker_that_ends_is_a_try_and_the_second_the_mark);
    run!(a_changed_file_makes_a_new_preview);
    run!(a_cache_entry_that_does_not_read_is_made_again);
    run!(a_fonts_preview_is_no_verdict);
    run!(the_previews_of_files_that_are_gone_are_swept);
    run!(the_cache_is_pruned_oldest_first);
    run!(a_nearly_full_storage_makes_no_preview);
    run!(the_request_queues_a_job_a_child_runs);
    run!(previews_wait_behind_the_other_jobs);
    run!(no_worker_writes_nothing);
    done();
}
