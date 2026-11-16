// test_file_previews.rs: Files' previews (2.2, issue #145; src/bgwork_previews.rs), the job as the background child
// runs it, on a storage tree of the host's own. Every preview goes through a real worker, a fresh exec of the headless
// runner (GRIMOIRE_WORK_EXE), as in the app: a PDF's first page, a picture, a font's sample. A file the worker refuses
// or that takes too long is marked and not tried again; a file changed since it was listed gets nothing until it is
// listed again, and then a preview of its own; with no worker to be had nothing is written (and nothing decoded in the
// writer). The cache's prune, through the job. The Linux sandbox's steps need root and run in the CI job
// (test_sandbox_linux.rs, `previews_are_made_in_the_real_sandbox`).
#![cfg(feature = "sim")]
mod fontfix;
mod pdffix;
mod testlib;
use grimoire::bgwork_previews::*;
use grimoire::fsview::*;
use grimoire::prelude::*;
use testlib::*;

const HOOKS: [&str; 12] = [
    "GRIMOIRE_STATE",
    "GRIMOIRE_FONT_GLYPH_US",
    "GRIMOIRE_FONT_TEST_SLOW",
    "GRIMOIRE_PIC_TEST_HANG",
    "GRIMOIRE_PIC_WATCHDOG_MS",
    "GRIMOIRE_FONT_TEST_HANG",
    "GRIMOIRE_FONT_WATCHDOG_MS",
    "GRIMOIRE_PDF_WATCHDOG_MS",
    "GRIMOIRE_PREVIEWS_MAX",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_PDF_NO_MAP",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
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

fn cache_names(g: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(format!("{g}/.cache/files")).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
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
        check!(info == format!("grimoire-preview 1\nof {} {}\nkind {kind}\nmade 300 400\n", s.size, s.mtime));
        check!(preview_info(&sp(&p), &s).is_some_and(|i| i.made == Ok((300, 400))));
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

// A file the worker refuses gets a "no preview" mark (its .info, no picture), kept: the next job for it finds the mark
// and runs no worker. A PDF that is no PDF, a PNG that is no picture, a web font.
fn a_refused_file_is_marked_and_not_tried_again() {
    clean_env();
    let (root, g) = tree();
    for name in ["junk.pdf", "broken.png", "web.ttf"] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        check!(matches!(preview_cached(&sp(&p), &s), Cached::NoPreview));
        check!(preview_info(&sp(&p), &s).is_some_and(|i| i.made == Err(NoPreview::Refused)));
        let key = preview_key(p.as_bytes(), s.size, s.mtime);
        check!(!std::path::Path::new(&preview_file(key, "pgm")).exists());
        std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
        check!(bg_file_preview(&arg(&s, &p)) == 0);
        std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    }
    check!(cache_names(&g).len() == 3);
}

// A worker that never answers is killed by its watchdog: the file is marked as taking too long (a picture after a
// fresh worker's second try, as every picture is; a font at once).
fn a_file_that_takes_too_long_is_marked() {
    clean_env();
    let (root, g) = tree();
    std::env::set_var("GRIMOIRE_PIC_TEST_HANG", "1");
    std::env::set_var("GRIMOIRE_PIC_WATCHDOG_MS", "400");
    let p = "documents/Covers/photo.png";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_info(&sp(p), &s).is_some_and(|i| i.made == Err(NoPreview::Slow)));
    std::env::set_var("GRIMOIRE_FONT_TEST_HANG", "0");
    std::env::set_var("GRIMOIRE_FONT_WATCHDOG_MS", "400");
    let p = "documents/Covers/face.ttf";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_info(&sp(p), &s).is_some_and(|i| i.made == Err(NoPreview::Slow)));
    clean_env();
}

// A file changed since it was listed (another picture written over it): the file listed again has another key, and
// its job makes a preview of its own, of the new picture; the old listing's preview stays (its job finds it there and
// runs no worker) until the prune. A file that changes between its listing and its job: nothing is written.
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
    let later = |path: &str, mtime: i64| {
        let t = std::ffi::CString::new(path).unwrap();
        let times = [libc::timeval { tv_sec: mtime as libc::time_t, tv_usec: 0 }; 2];
        // SAFETY: utimes reads the path and the two times we own.
        unsafe { libc::utimes(t.as_ptr(), times.as_ptr()) == 0 }
    };
    check!(later(&path, old.mtime + 5));
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
    check!(later(&format!("{root}/{f}"), s.mtime + 7));
    check!(bg_file_preview(&arg(&s, f)) == -1);
    check!(cache_names(&g).len() == 4);
    check!(bg_file_preview(&arg(&listed(&root, &g, f), f)) == 0);
    check!(cache_names(&g).len() == 6);
}

// A .info or picture that does not read is no preview: a .info of another file (its size), one cut short, a picture
// of another size than its .info says, a link in the cache's place; the next job makes it again.
fn a_cache_entry_that_does_not_read_is_made_again() {
    clean_env();
    let (root, g) = tree();
    let p = "documents/Covers/photo.png";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0);
    let key = preview_key(p.as_bytes(), s.size, s.mtime);
    let (info, pgm) = (preview_file(key, "info"), preview_file(key, "pgm"));
    let good = std::fs::read(&pgm).unwrap();
    std::fs::write(&pgm, &good[..good.len() - 1]).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    std::fs::write(&pgm, &good).unwrap();
    check!(preview_of(p, &s).is_some());
    std::fs::write(&info, format!("grimoire-preview 1\nof {} {}\nkind picture\nmade 300 400\n", s.size + 1, s.mtime)).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    std::fs::write(&info, format!("grimoire-preview 1\nof {} {}\nkind font\nmade 300 400\n", s.size, s.mtime)).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    std::fs::write(&info, format!("grimoire-preview 1\nof {} {}\nkind picture\nmade 300 399\n", s.size, s.mtime)).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    std::fs::remove_file(&info).unwrap();
    std::os::unix::fs::symlink(format!("{g}/settings.ini"), &info).unwrap();
    check!(matches!(preview_cached(&sp(p), &s), Cached::Nothing));
    std::fs::remove_file(&info).unwrap();
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_of(p, &s).is_some());
}

// A font's preview is no check of the font (#111): a font in grimoire/fonts, previewed, has no verdict, no entry in
// the fonts' own cache and no mark there: the folder where verdicts are kept stays empty, and grimoire/fonts holds
// the font file alone, so the font still waits for its check and nothing draws text with it. A font whose sample's
// letter is over the check's glyph limit has no preview (marked as taking too long), and no verdict either.
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
    // a letter of the sample over the glyph limit: no preview, marked as taking too long, and still no verdict
    std::fs::write(format!("{g}/fonts/Slow.ttf"), fontfix::ttf(&fontfix::test_font("Bold", 700))).unwrap();
    let slow = "grimoire/fonts/Slow.ttf";
    let s2 = listed(&root, &g, slow);
    std::env::set_var("GRIMOIRE_FONT_GLYPH_US", "1000");
    std::env::set_var("GRIMOIRE_FONT_TEST_SLOW", "from:0:5000");
    check!(bg_file_preview(&arg(&s2, slow)) == 0);
    check!(preview_info(&sp(slow), &s2).is_some_and(|i| i.made == Err(NoPreview::Slow)));
    check!(files_under(&state).is_empty());
    clean_env();
}

// The prune, through the job: past its limits (made small here) the oldest previews go, each with all of its files,
// down to three quarters; the one just made stays. Deleting the cache loses nothing: the next job makes it again.
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
            let f = std::ffi::CString::new(preview_file(preview_key(p.as_bytes(), s.size, s.mtime), ext)).unwrap();
            let t = [libc::timeval { tv_sec: (wall_time() - 7200 + 3600 * i as i64) as libc::time_t, tv_usec: 0 }; 2];
            // SAFETY: utimes reads the path and the two times we own.
            check!(unsafe { libc::utimes(f.as_ptr(), t.as_ptr()) } == 0);
        }
    }
    check!(cache_names(&g).len() == 4);
    // at most 4 files: the third preview makes 6, and the prune leaves 3 at most, the newest first
    std::env::set_var("GRIMOIRE_PREVIEWS_MAX", "4,100000000");
    let p = "documents/Covers/face.ttf";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == 0);
    let left = cache_names(&g);
    let stem = |p: &str| {
        let s = listed(&root, &g, p);
        format!("{:016x}", preview_key(p.as_bytes(), s.size, s.mtime))
    };
    check!(left.len() == 2);
    check!(left.iter().all(|n| n.starts_with(&stem("documents/Covers/face.ttf"))));
    clean_env();
    // the folder deleted: nothing lost, the next job makes the folder and the preview again
    std::fs::remove_dir_all(&dir).unwrap();
    check!(preview_of(p, &s).is_none());
    check!(bg_file_preview(&arg(&s, p)) == 0);
    check!(preview_of(p, &s).is_some());
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
    let t0 = grimoire::util::real_now_ms();
    while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 30_000 {
        bg_poll();
        sleep_ms(5);
    }
    check!(!bg_busy());
    check!(!bg_job_pending(BgKind::FilePreviews, "", j.as_deref().unwrap_or("")));
    check!(preview_of(p, &s).is_some());
}

// No worker to be had (none starts, or a sandbox step fails): nothing is written, no mark, and nothing is decoded in
// the writer; the next visit asks again. (Last: it leaves this process without picture workers.)
fn no_worker_writes_nothing() {
    clean_env();
    let (root, g) = tree();
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    for name in ["planner.pdf", "photo.png", "face.ttf"] {
        let p = format!("documents/Covers/{name}");
        let s = listed(&root, &g, &p);
        check!(bg_file_preview(&arg(&s, &p)) == -1);
        check!(matches!(preview_cached(&sp(&p), &s), Cached::Nothing));
    }
    clean_env();
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let p = "documents/Covers/photo.png";
    let s = listed(&root, &g, p);
    check!(bg_file_preview(&arg(&s, p)) == -1);
    check!(cache_names(&g).is_empty());
    clean_env();
}

#[test]
fn test_file_previews() {
    run!(a_pdf_a_picture_and_a_font_are_previewed);
    run!(a_refused_file_is_marked_and_not_tried_again);
    run!(a_file_that_takes_too_long_is_marked);
    run!(a_changed_file_makes_a_new_preview);
    run!(a_cache_entry_that_does_not_read_is_made_again);
    run!(a_fonts_preview_is_no_verdict);
    run!(the_cache_is_pruned_oldest_first);
    run!(the_request_queues_a_job_a_child_runs);
    run!(no_worker_writes_nothing);
    done();
}
