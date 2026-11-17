// test_template_pdfs.rs: PDFs as templates (2.2, issue #172; src/bgwork_tplpdf.rs): a PDF in grimoire/templates gives
// one template a page, its pages drawn once by the sandboxed PDF worker (a fresh exec of the headless runner,
// GRIMOIRE_WORK_EXE, as in the app) into the form a picture template takes, in the folder's cache. The scan's tiles
// (one waiting tile until the pages are counted, then one a page), the job's slices and where it ranks, the cap, the
// refusals, a page that fails alone, a worker that crashes or hangs, a PDF changed or swapped while its pages are made,
// no room, no worker; a page used as a notebook's template with the PDF gone; the Templates sheet reading the folder
// again once the pages are counted (never under a contact). The Linux sandbox's own steps run in the CI job as root
// (test_sandbox_linux.rs); here the worker is the same binary and protocol without them.
#![cfg(feature = "sim")]
mod pdffix;
mod testlib;
use grimoire::bgwork_template::preview_hash;
use grimoire::bgwork_tplpdf::*;
use grimoire::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::sync::OnceLock;
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
fn root() -> String {
    ROOT.get().cloned().unwrap_or_default()
}
const DPI: f32 = 300.0;
// the page size the store gets (store_set_device in the test's main): small, so the pages draw fast
const PW: i32 = 600;
const PH: i32 = 800;

const HOOKS: [&str; 10] = [
    "GRIMOIRE_PDF_TEST_CRASH_PAGE",
    "GRIMOIRE_PDF_TEST_HANG_PAGE",
    "GRIMOIRE_PDF_WATCHDOG_MS",
    "GRIMOIRE_PDF_SLICE_PAGES",
    "GRIMOIRE_PDF_FREE_BYTES",
    "GRIMOIRE_PDF_NO_MAP",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_BG_DELAY_MS",
    "GRIMOIRE_PDF_BANDS",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    sim_after_page_clear();
}

fn tpl() -> String {
    format!("{}/templates", root())
}
fn cache() -> String {
    format!("{}/templates/.cache", root())
}

/// The templates folder made afresh (its cache too) with these files in it.
fn folder(files: &[(&str, Vec<u8>)]) {
    settle();
    let _ = std::fs::remove_dir_all(tpl());
    std::fs::create_dir_all(tpl()).unwrap();
    for (n, b) in files {
        std::fs::write(format!("{}/{n}", tpl()), b).unwrap();
    }
}

/// The cache's entries, by name.
fn cache_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(cache())
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.')).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// What a file's cache entries begin with, as it is now.
fn hash(name: &str) -> String {
    let md = std::fs::metadata(format!("{}/{name}", tpl())).unwrap();
    hash_hex(preview_hash(name, md.len(), md.mtime()))
}

fn job(name: &str) -> i32 {
    bg_template("", &format!("{}/{name}", tpl()), DPI)
}

/// The job run slice after slice until it ends: its last result and how many slices it took.
fn job_to_end(name: &str) -> (i32, usize) {
    let mut n = 0;
    loop {
        let rc = job(name);
        n += 1;
        if rc != CODE_MORE || n > 200 {
            return (rc, n);
        }
    }
}

fn settle() {
    let t0 = grimoire::util::real_now_ms();
    while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 120_000 {
        let mut kind = -1;
        bg_poll_kind(Some(&mut kind));
        if kind == BgKind::Template as i32 {
            bgimage_retry_missing(); // (as app.rs does when one ends)
        }
        sleep_ms(5);
    }
    check!(!bg_busy());
}

fn scan() -> Vec<Template> {
    let mut t = Vec::new();
    check!(template_scan(&mut t, DPI) == 0);
    t
}

fn names(t: &[Template]) -> Vec<String> {
    t.iter().map(|t| t.name.as_str().to_string()).collect()
}

/// A PNG's width, height and colour type (0 gray, 2 RGB), from its header.
fn png_head(path: &str) -> (u32, u32, u8) {
    let d = std::fs::read(path).unwrap_or_default();
    if d.len() < 26 {
        return (0, 0, 255);
    }
    let be = |i: usize| u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
    (be(16), be(20), d[25])
}

/// Runs `f` with this process's log (stderr) kept in a file: what it returned, and the log's text.
fn with_log<R>(name: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let path = format!("{}/tp-{name}.log", tl_tmp());
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

// ---- the render path ----

// A PDF of three pages: until its pages are counted the scan shows one tile for it, waiting, and queues its job (no
// job for the folder's previews: it has no picture). The job draws each page once, at the page size, into the cache
// (the page's picture, its key, its preview, and the count); the scan then shows a tile a page, "Planner p. 1" to
// "p. 3", each ready, each its picture's path, and queues nothing. The key is the hash of the picture's bytes and its
// size, as a picture template's is, and the preview is a sixth of the page.
fn a_pdf_becomes_a_template_a_page() {
    clean_env();
    folder(&[("Planner.pdf", pdffix::planner(3))]);
    let path = format!("{}/Planner.pdf", tpl());
    let t = scan();
    check!(names(&t) == ["Planner"] && !t[0].ready && !t[0].bad && t[0].page == 0 && t[0].path.as_str() == path);
    check!(bg_job_pending(BgKind::Template, "", &path) && !bg_job_pending(BgKind::Template, "", &tpl()));
    settle();
    let h = hash("Planner.pdf");
    let mut want: Vec<String> = (1..=3).flat_map(|k| ["key", "pgm", "png"].map(|e| format!("{h}-p{k:04}.{e}"))).collect();
    want.push(format!("{h}.pages"));
    check!(cache_names() == want);
    check!(std::fs::read(format!("{}/{h}.pages", cache())).unwrap() == b"3\n");
    let mut pngs = Vec::new();
    for k in 1..=3u32 {
        let png = page_file(&cache(), &h, k, "png");
        check!(png_head(&png) == (PW as u32, PH as u32, 0));
        let bytes = std::fs::read(&png).unwrap();
        let key = std::fs::read_to_string(page_file(&cache(), &h, k, "key")).unwrap();
        check!(key == format!("{:016x} {PW} {PH}\n", bgimage_content_hash(&bytes)));
        pngs.push(bytes);
    }
    check!(pngs[0] != pngs[1] && pngs[1] != pngs[2]);
    let mut t = scan();
    check!(names(&t) == ["Planner p. 1", "Planner p. 2", "Planner p. 3"]);
    check!(t.iter().enumerate().all(|(i, t)| t.ready && !t.bad && t.page == i as u32 + 1 && t.path.as_str() == page_file(&cache(), &h, i as u32 + 1, "png")));
    check!(!bg_busy());
    let th = template_thumb(&mut t[1]).map(|c| (c.w, c.h));
    check!(th == Some((PW / 6, PH / 6)));
    // the page's picture is the page's own cache file, its preview and key beside it
    check!(page_stem(t[1].path.as_str()) == Some(page_file(&cache(), &h, 2, "png").trim_end_matches(".png")));
    check!(page_stem(&format!("{}/Planner.pdf", tpl())).is_none() && page_stem(&format!("{}/x/{h}-p0002.png", root())).is_none());
    template_list_free(&mut t);
    // asked again: done, no worker started (none can be: it is not needed)
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(job("Planner.pdf") == 0);
    clean_env();
}

// A page is a template as a picture is: installed into a notebook, its picture is made for the notebook's page in the
// background, from the page's picture in the cache. The PDF can be gone by then: drawing a page never reads the PDF.
// A colour page stays colour (the PDF's gray pages gray), as a picture template's colour does.
fn a_page_is_used_as_a_picture_template_is() {
    clean_env();
    folder(&[("Colour.pdf", pdffix::colour())]);
    check!(job_to_end("Colour.pdf").0 == 0);
    let h = hash("Colour.pdf");
    check!((1..=3u32).map(|k| png_head(&page_file(&cache(), &h, k, "png")).2).collect::<Vec<_>>() == [0, 2, 0]);
    let t = scan();
    check!(names(&t) == ["Colour p. 1", "Colour p. 2", "Colour p. 3"]);
    let page2 = t[1].path.as_str().to_string();
    std::fs::remove_file(format!("{}/Colour.pdf", tpl())).unwrap();
    let mut nb = NotebookInfo::default();
    check!(store_create("On a PDF page", "", PAPER_BLANK, &mut nb) == 0);
    let mut kind = PaperKind::default();
    let mut why: &'static str = "";
    check!(template_install(&nb, &page2, &mut kind, &mut why) == 0);
    check!(paper_is_image(kind) && !paper_is_document(kind));
    settle();
    let mut dir = String::new();
    check!(bgimage_dir(nb.id.as_str(), &mut dir, 700) == 0);
    let mut file = String::new();
    bgimage_file(&dir, paper_image_id(kind), &mut file, 800);
    check!(png_head(&file) == (PW as u32, PH as u32, 2));
    // the paper the page is, by the notebook's index, as the sheet selects it
    let mut ix = BgIndex::default();
    check!(bgindex_load(&dir, &mut ix) == 0);
    let mut k2 = PaperKind::default();
    check!(template_paper(&t[1], &nb, &ix, &mut k2) == 0 && k2 == kind);
    check!(template_paper(&t[0], &nb, &ix, &mut k2) == -1);
    bgindex_free(&mut ix);
}

// A PDF's pages are made in slices, as an import's: here two pages a slice, so five pages take three, the first two
// ending "more to do". Through the queue the same: each slice that leaves pages is queued again until the last.
fn the_pages_come_in_slices() {
    clean_env();
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "2");
    folder(&[("Five.pdf", pdffix::planner(5))]);
    let h = hash("Five.pdf");
    let made = || cache_names().iter().filter(|n| n.ends_with(".pgm")).count();
    let (rc, log) = with_log("slices", || job("Five.pdf"));
    check!(rc == CODE_MORE && made() == 2 && log.contains("a PDF of 5 page(s)") && log.contains("slice made 2 page(s)"));
    check!(job("Five.pdf") == CODE_MORE && made() == 4);
    check!(job("Five.pdf") == 0 && made() == 5);
    check!(std::fs::read(format!("{}/{h}.pages", cache())).unwrap() == b"5\n");
    // the queue: one job, queued again behind whatever ranks higher, until its last slice
    folder(&[("Five.pdf", pdffix::planner(5))]);
    let t = scan();
    check!(names(&t) == ["Five"]);
    settle();
    check!(made() == 5 && scan().iter().all(|t| t.ready) && scan().len() == 5);
    clean_env();
}

// The cap: a PDF of 51 pages is no template as a whole (never its first 50): marked, nothing else made, and the scan
// shows one tile for it, bad (the sheet leaves it out and counts it among the files it cannot use). A PDF of exactly 50
// is taken. Past 16 MB, or empty, a PDF is marked before any worker starts.
fn the_page_cap_and_the_size() {
    clean_env();
    check!(TEMPLATE_PDF_MAX_PAGES == 50);
    folder(&[("Long.pdf", pdffix::planner(51))]);
    let h = hash("Long.pdf");
    let (rc, log) = with_log("cap", || job("Long.pdf"));
    check!(rc == 0 && cache_names() == [format!("{h}.bad")]);
    check!(log.contains("51 pages, more than the 50 a template PDF may have"));
    let t = scan();
    check!(t.len() == 1 && t[0].bad && names(&t) == ["Long"] && !bg_busy());
    // exactly the cap: counted, and its first slice makes its first page
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    folder(&[("Fifty.pdf", pdffix::planner(50))]);
    let h = hash("Fifty.pdf");
    check!(job("Fifty.pdf") == CODE_MORE);
    check!(std::fs::read(format!("{}/{h}.pages", cache())).unwrap() == b"50\n");
    check!(pdf_state(&cache(), &h) == PdfPages::Known(50));
    std::env::remove_var("GRIMOIRE_PDF_SLICE_PAGES");
    // too big, and empty: marked with no worker (none could start here)
    let mut big = pdffix::planner(1);
    big.resize(16 * 1024 * 1024 + 1, b' ');
    folder(&[("Big.pdf", big), ("Empty.pdf", Vec::new())]);
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let (hb, he) = (hash("Big.pdf"), hash("Empty.pdf"));
    let (rc, log) = with_log("big", || (job("Big.pdf"), job("Empty.pdf")));
    let mut want = vec![format!("{hb}.bad"), format!("{he}.bad")];
    want.sort();
    check!(rc == (0, 0) && cache_names() == want);
    check!(log.contains("a PDF over 16 MB") && log.contains("an empty file"));
    check!(scan().iter().all(|t| t.bad));
    clean_env();
}

// What the worker refuses is marked as a whole: a file that is no PDF, a PDF with a password. Nothing else is made.
fn refused_pdfs_are_marked() {
    clean_env();
    folder(&[("Junk.pdf", pdffix::junk()), ("Locked.pdf", pdffix::encrypted_planner(2, "secret", "owner"))]);
    let (hj, hl) = (hash("Junk.pdf"), hash("Locked.pdf"));
    let (rc, log) = with_log("refused", || (job("Junk.pdf"), job("Locked.pdf")));
    check!(rc == (0, 0));
    let mut want = vec![format!("{hj}.bad"), format!("{hl}.bad")];
    want.sort();
    check!(cache_names() == want);
    check!(log.contains("(no PDF Grimoire can read); marked") && log.contains("(it needs a password); marked"));
    let t = scan();
    check!(t.len() == 2 && t.iter().all(|t| t.bad) && !bg_busy());
}

// A page that ends its worker (here it crashes it, every time) is tried once more by a fresh worker, then marked
// alone: the PDF's other pages are made, and the sheet offers them. One that stalls is ended by the page watchdog and
// marked the same way. Two such pages make the PDF no template; so does a PDF none of whose pages draws.
fn a_page_that_fails_is_marked_alone() {
    clean_env();
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "2");
    folder(&[("Three.pdf", pdffix::planner(3))]);
    let h = hash("Three.pdf");
    let ((rc, _), log) = with_log("crash", || job_to_end("Three.pdf"));
    check!(rc == 0);
    check!(std::fs::read(page_file(&cache(), &h, 2, "bad")).unwrap() == b"died\n");
    check!(log.contains("page 2 of a PDF: the worker died of signal") && log.contains("(strike 1)") && log.contains("(strike 2)"));
    let t = scan();
    check!(names(&t) == ["Three p. 1", "Three p. 2", "Three p. 3"]);
    check!(t[0].ready && t[1].bad && !t[1].ready && t[2].ready && !bg_busy());
    clean_env();
    // a page that hangs its worker: the watchdog (here 1 s) ends it, twice
    std::env::set_var("GRIMOIRE_PDF_TEST_HANG_PAGE", "1");
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "1000");
    folder(&[("Two.pdf", pdffix::planner(2))]);
    let h = hash("Two.pdf");
    let started = std::time::Instant::now();
    let ((rc, _), log) = with_log("hang", || job_to_end("Two.pdf"));
    check!(rc == 0 && started.elapsed().as_secs() < 20);
    check!(log.contains("page 1 of a PDF: the worker stalled (strike 2)"));
    check!(std::fs::read(page_file(&cache(), &h, 1, "bad")).unwrap() == b"died\n" && std::path::Path::new(&page_file(&cache(), &h, 2, "pgm")).exists());
    // two pages so: the whole PDF is no template, and what was made of it goes
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "2");
    folder(&[("Hostile.pdf", pdffix::planner(3))]);
    let h = hash("Hostile.pdf");
    let ((rc, _), log) = with_log("hostile", || job_to_end("Hostile.pdf"));
    check!(rc == 0 && cache_names() == [format!("{h}.bad")]);
    check!(log.contains("2 of its pages ended or stalled their worker twice"));
    check!(scan().iter().all(|t| t.bad));
    clean_env();
    // a PDF of one page that does not draw
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "1");
    folder(&[("One.pdf", pdffix::planner(1))]);
    let h = hash("One.pdf");
    let ((rc, _), log) = with_log("none", || job_to_end("One.pdf"));
    check!(rc == 0 && cache_names() == [format!("{h}.bad")] && log.contains("none of its pages could be drawn"));
    clean_env();
}

// The writer without memory for a page's pixels (here a page size no memory holds): not the file's doing, so no strike
// and no mark (two pages so would otherwise refuse the whole PDF for good); the slice stops, and a later look, with the
// memory there, draws the pages.
fn the_writer_out_of_memory_marks_nothing() {
    clean_env();
    folder(&[("Two.pdf", pdffix::planner(2))]);
    let h = hash("Two.pdf");
    store_set_device(DPI, 1 << 30, 1 << 30);
    let (rc, log) = with_log("nomem", || job("Two.pdf"));
    store_set_device(DPI, PW, PH);
    check!(rc == -1 && log.contains("page 1 of a PDF left the writer without memory") && !log.contains("(strike"));
    check!(cache_names() == [format!("{h}.pages")]);
    check!(job_to_end("Two.pdf").0 == 0 && scan().iter().filter(|t| t.ready).count() == 2);
}

// A PDF swapped while its pages are made (another file renamed onto its name after page 1), or written over in place:
// no page of the file after the change is kept, what was made of the old one goes, nothing is marked, and the next look
// makes the pages of the file as it is now.
fn a_pdf_changed_while_its_pages_are_made() {
    clean_env();
    for in_place in [false, true] {
        folder(&[("Swap.pdf", pdffix::planner(3))]);
        let old = hash("Swap.pdf");
        let dir = tpl();
        sim_after_page(move |k| {
            if k == 1 {
                let to = format!("{dir}/Swap.pdf");
                if in_place {
                    std::fs::write(&to, pdffix::planner(4)).unwrap();
                } else {
                    std::fs::write(format!("{dir}/.next"), pdffix::planner(4)).unwrap();
                    std::fs::rename(format!("{dir}/.next"), &to).unwrap();
                }
            }
        });
        let (rc, log) = with_log("swap", || job("Swap.pdf"));
        sim_after_page_clear();
        check!(rc == -1 && log.contains("a PDF changed while its pages were made"));
        check!(cache_names().iter().all(|n| !n.starts_with(&old)));
        // the file as it is now: its own four pages
        let t = scan();
        check!(names(&t) == ["Swap"] && !t[0].bad);
        settle();
        let t = scan();
        check!(names(&t) == ["Swap p. 1", "Swap p. 2", "Swap p. 3", "Swap p. 4"] && t.iter().all(|t| t.ready));
    }
    // written over right after its pages were counted, before any page: the count is not kept (nor said), nothing is
    // made or marked
    folder(&[("Swap.pdf", pdffix::planner(3))]);
    let dir = tpl();
    sim_after_page(move |k| {
        if k == 0 {
            std::fs::write(format!("{dir}/Swap.pdf"), pdffix::planner(4)).unwrap();
        }
    });
    let (rc, log) = with_log("swap-count", || job("Swap.pdf"));
    sim_after_page_clear();
    check!(rc == -1 && log.contains("a PDF changed while its pages were made"));
    check!(!log.contains("a PDF of 3 page(s)") && cache_names().is_empty());
    clean_env();
}

// No room above the floor: nothing made, nothing marked (not the file's fault); the next look tries again.
fn no_room_makes_nothing() {
    clean_env();
    std::env::set_var("GRIMOIRE_PDF_FREE_BYTES", "50000000");
    folder(&[("Planner.pdf", pdffix::planner(2))]);
    let (rc, log) = with_log("room", || job("Planner.pdf"));
    check!(rc == -1 && cache_names().is_empty() && log.contains("not enough room on the storage"));
    check!(names(&scan()) == ["Planner"]);
    settle();
    check!(cache_names().is_empty());
    // no room at a later slice: what the first made is kept (the count and page 1), nothing more is made or marked
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    clean_env_keep_slices();
    check!(job("Planner.pdf") == CODE_MORE && cache_names().len() == 4);
    std::env::set_var("GRIMOIRE_PDF_FREE_BYTES", "50000000");
    let (rc, log) = with_log("room-page", || job("Planner.pdf"));
    check!(rc == -1 && cache_names().len() == 4 && log.contains("not enough room on the storage"));
    clean_env();
    check!(job_to_end("Planner.pdf").0 == 0 && scan().len() == 2);
    // no room from the middle of a slice on (after page 1 of a slice that could take all three): the floor is looked
    // at again before each page's worker is asked, so page 2's is never asked (here it would crash it, twice, and the
    // page would be marked); page 1 is kept, page 2 neither drawn nor marked
    std::env::set_var("GRIMOIRE_PDF_TEST_CRASH_PAGE", "2");
    folder(&[("Three.pdf", pdffix::planner(3))]);
    let h = hash("Three.pdf");
    sim_after_page(|k| {
        if k == 1 {
            std::env::set_var("GRIMOIRE_PDF_FREE_BYTES", "50000000");
        }
    });
    let (rc, log) = with_log("room-in-slice", || job("Three.pdf"));
    sim_after_page_clear();
    check!(rc == -1 && log.contains("not enough room on the storage") && !log.contains("page 2 of a PDF"));
    check!(cache_names() == [format!("{h}-p0001.key"), format!("{h}-p0001.pgm"), format!("{h}-p0001.png"), format!("{h}.pages")]);
    clean_env();
}

/// clean_env, with the slice length given kept.
fn clean_env_keep_slices() {
    let slices = std::env::var("GRIMOIRE_PDF_SLICE_PAGES").ok();
    clean_env();
    if let Some(v) = slices {
        std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", v);
    }
}

// A PDF in grimoire/templates is never an import: the import watch reads import/ alone.
fn a_template_pdf_is_never_imported() {
    clean_env();
    folder(&[("Planner.pdf", pdffix::planner(2))]);
    let f = grimoire::pdfimport::list_pdfs();
    check!(f.files.is_empty() && f.pdf_folders.is_empty());
    let mut all = Vec::new();
    store_list(&mut all);
    let before = all.len();
    check!(job_to_end("Planner.pdf").0 == 0);
    store_list(&mut all);
    check!(all.len() == before);
}

// A folder of pictures alone is read as it was: no tile is a page, no PDF job is queued.
fn pictures_alone_are_as_they_were() {
    clean_env();
    let mut p = BgPicture { px: vec![200; 60 * 80], w: 60, h: 80, ch: 1 };
    let path = format!("{}/x.png", tl_tmp());
    check!(bgimage_write_png(&path, &p) == 0);
    let png = std::fs::read(&path).unwrap();
    bgpicture_free(&mut p);
    folder(&[("Lined.png", png.clone()), ("Dots.png", png)]);
    let t = scan();
    check!(names(&t) == ["Dots", "Lined"] && t.iter().all(|t| t.page == 0));
    check!(bg_job_pending(BgKind::Template, "", &tpl()));
    settle();
    check!(scan().iter().all(|t| t.ready && t.page == 0) && !bg_busy());
}

/// A small PNG's bytes (60 x 80, gray).
fn png_bytes() -> Vec<u8> {
    let mut p = BgPicture { px: vec![200; 60 * 80], w: 60, h: 80, ch: 1 };
    let path = format!("{}/x.png", tl_tmp());
    check!(bgimage_write_png(&path, &p) == 0);
    bgpicture_free(&mut p);
    std::fs::read(&path).unwrap()
}

// A new picture beside a PDF: the folder's previews job (ranked first) makes the picture's preview and leaves the PDF
// to its own job (it never hands a PDF to the picture worker, which would find it no picture and mark it for good),
// so the PDF's pages are made too.
fn a_picture_beside_a_pdf() {
    clean_env();
    folder(&[("Lined.png", png_bytes()), ("Plan.pdf", pdffix::planner(2))]);
    let h = hash("Plan.pdf");
    let t = scan();
    check!(names(&t) == ["Lined", "Plan"] && t.iter().all(|t| !t.ready && !t.bad));
    check!(bg_job_pending(BgKind::Template, "", &tpl()) && bg_job_pending(BgKind::Template, "", &format!("{}/Plan.pdf", tpl())));
    settle();
    check!(!std::path::Path::new(&format!("{}/{h}.bad", cache())).exists());
    let t = scan();
    check!(names(&t) == ["Lined", "Plan p. 1", "Plan p. 2"] && t.iter().all(|t| t.ready && !t.bad));
}

// Where a PDF's slices run: behind a picture the user picked (a template's picture for a notebook) and behind a font
// check, though queued before them; a slice that leaves pages is queued again behind the jobs queued meanwhile.
fn the_pages_wait_behind_other_jobs() {
    clean_env();
    std::env::set_var("GRIMOIRE_PDF_SLICE_PAGES", "1");
    std::fs::create_dir_all(format!("{}/fonts", root())).unwrap();
    folder(&[("A.pdf", pdffix::planner(2)), ("B.pdf", pdffix::planner(1))]);
    std::env::set_var("GRIMOIRE_BG_DELAY_MS", "300");
    let ((), log) = with_log("order", || {
        bg_queue(BgKind::Template, Some(""), Some(&format!("{}/A.pdf", tpl())), DPI, false);
        bg_queue(BgKind::Template, Some(""), Some(&format!("{}/B.pdf", tpl())), DPI, false);
        bg_queue(BgKind::FontCheck, None, Some(""), DPI, false);
        bg_queue(BgKind::Template, Some("no-such-notebook"), Some("/nonexistent.png"), DPI, true);
        settle();
    });
    std::env::remove_var("GRIMOIRE_BG_DELAY_MS");
    let started: Vec<&str> = log
        .lines()
        .filter(|l| l.contains(" (pid ") && l.contains("bg: "))
        .map(|l| if l.contains("template of a file") { "pdf" } else if l.contains("font check of") { "fonts" } else if l.contains("template of no-such-notebook") { "picture" } else { "other" })
        .collect();
    // A's first slice (it was alone), the picture, the font check, B, then A's second slice
    check!(started == ["pdf", "picture", "fonts", "pdf", "pdf"]);
    check!(scan().iter().filter(|t| t.ready).count() == 3);
    clean_env();
}

// The Templates sheet over a folder with a PDF: one waiting tile for it; once its job has counted the pages the folder
// is read again and its pages take its place, but never under a contact: after the lift. Their previews fill in.
fn the_sheet_reads_the_folder_again_once_the_pages_are_counted() {
    clean_env();
    folder(&[("Planner.pdf", pdffix::planner(2))]);
    let mut a = App { dpi: DPI, ..Default::default() };
    a.w = 1860;
    a.h = 2480;
    check!(canvas_init(&mut a.screen, a.w, a.h) == 0);
    let mut s = TplSheet::default();
    tplsheet_open(&mut s, &mut a, None, PaperKind::default(), None);
    tplsheet_draw(&mut s, &mut a);
    let probe = |s: &TplSheet, k: &str| {
        let mut out = String::new();
        tplsheet_probe(s, k, &mut out, 256);
        out
    };
    check!(probe(&s, "names") == "Planner" && probe(&s, "count") == "1" && probe(&s, "ready") == "0");
    // a contact down on the sheet (on no tile) while the job counts and makes the pages
    let (x, y) = ((s.rect.x + 20) as f32, (s.rect.y + 20) as f32);
    let mut d = Rect::default();
    tplsheet_pointer(&mut s, &mut a, 0, x, y, 1000, &mut d);
    settle();
    check!(!tplsheet_tick(&mut s, &mut a, now_ms(), &mut d) || probe(&s, "names") == "Planner");
    check!(probe(&s, "names") == "Planner" && s.pdf_counted);
    check!(tplsheet_pointer(&mut s, &mut a, 2, x, y, 1080, &mut d) == TsResult::None && s.open);
    // the lift: read again, its pages in its place, ready
    check!(tplsheet_tick(&mut s, &mut a, now_ms(), &mut d) && !rect_is_empty(d));
    check!(probe(&s, "names") == "Planner p. 1|Planner p. 2" && probe(&s, "count") == "2" && probe(&s, "ready") == "2");
    tplsheet_close(&mut s);
    canvas_free(&mut a.screen);
}

// No worker to be had (none starts, or a sandbox step fails where the sandbox is required): nothing is made and
// nothing marked; after a sandbox step failed, this session queues no PDF's job. (Last: it leaves the session broken.)
fn no_worker_makes_nothing() {
    clean_env();
    folder(&[("Planner.pdf", pdffix::planner(2))]);
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let (rc, log) = with_log("noworker", || job("Planner.pdf"));
    check!(rc == -1 && cache_names().is_empty() && log.contains("no worker"));
    clean_env();
    // the sandbox required, and a worker without its steps (the host has none of them): no worker either
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    let (rc, log) = with_log("unsandboxed", || job("Planner.pdf"));
    check!(rc == -1 && cache_names().is_empty() && log.contains("no worker (the worker is not sandboxed)"));
    check!(!grimoire::sandbox::session_broken());
    clean_env();
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    let (rc, log) = with_log("nosandbox", || job("Planner.pdf"));
    check!(rc == -1 && cache_names().is_empty() && log.contains("the sandbox refused its step"));
    check!(grimoire::sandbox::session_broken());
    let t = scan();
    check!(names(&t) == ["Planner"] && !t[0].bad && !bg_busy());
    clean_env();
}

#[test]
fn test_template_pdfs() {
    let top = format!("{}/tplpdf-{}", tl_tmp(), std::process::id());
    let _ = ROOT.set(top.clone());
    let _ = std::fs::remove_dir_all(&top);
    check!(store_init(&top) == 0);
    store_set_device(DPI, PW, PH);
    templates_init();
    i18n_set("en");
    ui_init(DPI);
    check!(font_init() == 0);
    run!(a_pdf_becomes_a_template_a_page);
    run!(a_page_is_used_as_a_picture_template_is);
    run!(the_pages_come_in_slices);
    run!(the_page_cap_and_the_size);
    run!(refused_pdfs_are_marked);
    run!(a_page_that_fails_is_marked_alone);
    run!(the_writer_out_of_memory_marks_nothing);
    run!(a_pdf_changed_while_its_pages_are_made);
    run!(no_room_makes_nothing);
    run!(a_template_pdf_is_never_imported);
    run!(pictures_alone_are_as_they_were);
    run!(a_picture_beside_a_pdf);
    run!(the_pages_wait_behind_other_jobs);
    run!(the_sheet_reads_the_folder_again_once_the_pages_are_counted);
    run!(no_worker_makes_nothing);
    font_shutdown();
    done();
    let _ = std::fs::remove_dir_all(&top);
}
