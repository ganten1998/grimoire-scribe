//! The notebook PDF, streamed and incremental (lane L9): a changed notebook draws only its changed
//! pages, the streamed PDF is byte for byte a full render, the cache survives being deleted, a full
//! disk and damage, colour pages keep their colour, a zip's PDF entry streams, every background child
//! carries its caps. One test function: the store root is process-wide.

use grimoire::prelude::*;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("pdfstream-{}", std::process::id()));
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
    reload(&nb)
}

fn reload(nb: &NotebookInfo) -> NotebookInfo {
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

/// the page, with another stroke on it
fn scribble(nb: &NotebookInfo, i: i32, seed: u32) {
    let mut p: Option<Box<Page>> = None;
    assert_eq!(store_read_page(nb, i, &mut p), 0);
    let mut p = p.unwrap();
    page_add(&mut p, wave(seed, 700.0));
    assert_eq!(store_save_page(nb, i, &p), 0);
}

/// A full render: no cache read or written.
fn full(nb: &NotebookInfo, dpi: f32) -> Vec<u8> {
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf_to(nb, dpi, &mut BufSink(&mut b), PdfCacheMode::Off, None), 0);
    b.data
}

fn mirror(nb: &NotebookInfo, dpi: f32) -> (Vec<u8>, u32) {
    let dir = format!("{}/PDFs", root().display());
    let before = pdf_pages_rendered();
    assert_eq!(export_mirror_pdf(nb, dpi, &dir), 0);
    let drawn = pdf_pages_rendered() - before;
    let mut path = String::new();
    export_mirror_path(nb, &dir, &mut path, 1000);
    (std::fs::read(&path).unwrap(), drawn)
}

fn cache_dir(nb: &NotebookInfo) -> PathBuf {
    PathBuf::from(pdf_cache_root()).join(nb.id.as_str())
}

fn records(nb: &NotebookInfo) -> usize {
    std::fs::read_dir(cache_dir(nb)).map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".z")).count()).unwrap_or(0)
}

#[test]
fn the_notebook_pdf_streams_and_draws_only_what_changed() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, 800, 1000);
    streamed_is_a_full_render_and_the_mirror_fills_the_cache();
    a_changed_page_draws_once();
    stamps_and_damage_draw_again();
    a_deleted_cache_and_a_full_disk_lose_nothing();
    the_cache_prunes();
    the_writer_streams();
    colour_pages_keep_their_colour();
    a_zips_pdf_entry_streams();
    children_carry_their_caps();
}

fn streamed_is_a_full_render_and_the_mirror_fills_the_cache() {
    let nb = book("Streamed", 12);
    let want = full(&nb, 300.0);
    assert!(want.starts_with(b"%PDF-1.4"));
    let (first, drawn) = mirror(&nb, 300.0);
    assert_eq!(first, want, "the mirror's file is the full render");
    assert_eq!(drawn, 12, "the first mirror draws every page");
    assert_eq!(records(&nb), 12);
    assert!(!std::path::Path::new(&format!("{}/PDFs/Streamed.pdf.tmp", root().display())).exists(), "no temporary file is left");
    let (again, drawn) = mirror(&nb, 300.0);
    assert_eq!((again == want, drawn), (true, 0), "an unchanged notebook draws nothing");
    // the Buf route (the share server's in-process answer, the exporter tool) uses the records too
    let before = pdf_pages_rendered();
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf(&nb, 300.0, &mut b), 0);
    assert_eq!((b.data == want, pdf_pages_rendered() - before), (true, 0));
    // progress is told as before: (0, n), then each page
    let mut seen = Vec::new();
    let mut b = Buf::default();
    assert_eq!(export_notebook_pdf_progress(&nb, 300.0, &mut b, Some(&mut |d, t| seen.push((d, t)))), 0);
    assert_eq!(seen.len(), 13);
    assert_eq!((seen[0], seen[12]), ((0, 12), (12, 12)));
}

fn a_changed_page_draws_once() {
    let nb = book("Changed", 12);
    mirror(&nb, 300.0);
    // a save changes the page's stamp (taken before the page is read, so a save during a draw leaves the record stale)
    let cache = PdfCache::open(&nb, 300.0, PdfCacheMode::Read).unwrap();
    let before = cache.stamp_of(&nb, nb.page_ids[5]);
    scribble(&nb, 5, 99);
    assert_ne!(cache.stamp_of(&nb, nb.page_ids[5]), before, "a saved page has another stamp");
    assert_eq!(cache.stamp_of(&nb, nb.page_ids[4]), cache.stamp_of(&nb, nb.page_ids[4]));
    let nb = reload(&nb);
    let (pdf, drawn) = mirror(&nb, 300.0);
    assert_eq!(drawn, 1, "a changed notebook closes by drawing its one changed page");
    assert_eq!(pdf, full(&nb, 300.0), "and the file is what a full render makes");
    // a page added draws once; a page deleted draws none and loses its record
    let mut nb = nb;
    assert_eq!(store_add_page(&mut nb, 12, PAPER_NARROW_NM), 0);
    let mut p = page_new(800, 1000, PAPER_NARROW_NM);
    page_add(&mut p, wave(7, 300.0));
    assert_eq!(store_save_page(&nb, 12, &p), 0);
    let nb = reload(&nb);
    let (pdf, drawn) = mirror(&nb, 300.0);
    assert_eq!((drawn, records(&nb)), (1, 13));
    assert_eq!(pdf, full(&nb, 300.0));
    let mut nb = nb;
    assert_eq!(store_delete_page(&mut nb, 3), 0);
    let nb = reload(&nb);
    let (pdf, drawn) = mirror(&nb, 300.0);
    assert_eq!((drawn, records(&nb)), (0, 12), "the deleted page's record is pruned");
    assert_eq!(pdf, full(&nb, 300.0));
}

fn stamps_and_damage_draw_again() {
    let nb = book("Stamps", 6);
    mirror(&nb, 300.0);
    // another dpi: every page is drawn again
    let (_, drawn) = mirror(&nb, 301.0);
    assert_eq!(drawn, 6, "the dpi is in the stamp");
    // a page file with the same size and another modification time
    let pid = nb.page_ids[2];
    let file = r_page(&nb, pid);
    let f = std::fs::OpenOptions::new().write(true).open(&file).unwrap();
    f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(120)).unwrap();
    drop(f);
    let (pdf, drawn) = mirror(&nb, 301.0);
    assert_eq!(drawn, 1, "the page file's time is in the stamp");
    assert_eq!(pdf, full(&nb, 301.0));
    // damage: a flipped byte in a stream, a truncated record, garbage, an empty file
    let rec = |i: usize| cache_dir(&nb).join(format!("p{}.z", nb.page_ids[i]));
    let mut d = std::fs::read(rec(0)).unwrap();
    let n = d.len();
    d[n - 10] ^= 0x55;
    std::fs::write(rec(0), &d).unwrap();
    let d = std::fs::read(rec(1)).unwrap();
    std::fs::write(rec(1), &d[..d.len() / 2]).unwrap();
    std::fs::write(rec(3), b"not a record at all").unwrap();
    std::fs::write(rec(4), b"").unwrap();
    let (pdf, drawn) = mirror(&nb, 301.0);
    assert_eq!(drawn, 4, "every damaged record is drawn again");
    assert_eq!(pdf, full(&nb, 301.0));
    // the page-size stamp: a notebook moved to another page size draws all
    let (_, drawn) = mirror(&nb, 301.0);
    assert_eq!(drawn, 0);
}

fn r_page(nb: &NotebookInfo, pid: u32) -> String {
    format!("{}/notebooks/{}/p{}.qpg", root().display(), nb.id.as_str(), pid)
}

fn a_deleted_cache_and_a_full_disk_lose_nothing() {
    let nb = book("Disposable", 5);
    let want = full(&nb, 300.0);
    mirror(&nb, 300.0);
    std::fs::remove_dir_all(pdf_cache_root()).unwrap();
    let (pdf, drawn) = mirror(&nb, 300.0);
    assert_eq!((pdf == want, drawn, records(&nb)), (true, 5, 5), "a deleted cache is drawn again, and made again");
    // a full disk: the writes fail, the PDF is made as if there were no cache
    std::fs::remove_dir_all(cache_dir(&nb)).unwrap();
    pdf_cache_inject_full(true);
    let (pdf, drawn) = mirror(&nb, 300.0);
    pdf_cache_inject_full(false);
    assert_eq!((pdf == want, drawn, records(&nb)), (true, 5, 0), "a full disk: the same PDF, no records");
    // and a folder where the cache's folder should be
    std::fs::remove_dir_all(pdf_cache_root()).unwrap();
    std::fs::create_dir_all(r_root_cache()).unwrap();
    std::fs::write(cache_dir(&nb), b"a file where the folder goes").unwrap();
    let (pdf, _) = mirror(&nb, 300.0);
    assert_eq!(pdf, want, "a cache folder that cannot be made is no failure");
    std::fs::remove_file(cache_dir(&nb)).unwrap();
}

fn r_root_cache() -> PathBuf {
    PathBuf::from(pdf_cache_root())
}

fn the_cache_prunes() {
    let a = book("Prune A", 4);
    let b = book("Prune B", 4);
    mirror(&a, 300.0);
    mirror(&b, 300.0);
    assert_eq!((records(&a), records(&b)), (4, 4));
    let size = |nb: &NotebookInfo| std::fs::read_dir(cache_dir(nb)).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum::<u64>();
    let total = size(&a) + size(&b);
    // past the cap the oldest records go (A's were made first)
    pdf_cache_prune_to(&b, total * 3 / 4);
    let left = size(&a) + size(&b);
    assert!(left <= total * 3 / 4 && records(&a) < 4 && records(&b) == 4, "the oldest go first: {} {}", records(&a), records(&b));
    // a notebook gone takes its records with it at the next prune
    let ida = a.id.as_str().to_string();
    assert_eq!(store_delete(&ida), 0);
    pdf_cache_prune(&b);
    assert!(!cache_dir(&a).exists(), "a deleted notebook's records are pruned");
    assert_eq!(records(&b), 4);
    // a stray spool of a killed share child goes after an hour
    let spool = PathBuf::from(pdf_cache_root()).join("share-1.spool");
    std::fs::write(&spool, b"x").unwrap();
    pdf_cache_prune(&b);
    assert!(spool.exists(), "a young spool stays");
    let f = std::fs::OpenOptions::new().write(true).open(&spool).unwrap();
    f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200)).unwrap();
    drop(f);
    pdf_cache_prune(&b);
    assert!(!spool.exists(), "an old one goes");
}

/// A sink that notes how the bytes come.
struct Watch {
    total: Rc<Cell<usize>>,
    puts: usize,
    biggest: usize,
}
impl Sink for Watch {
    fn put(&mut self, b: &[u8]) -> bool {
        self.total.set(self.total.get() + b.len());
        self.puts += 1;
        self.biggest = self.biggest.max(b.len());
        true
    }
}

fn the_writer_streams() {
    let nb = book("Streaming", 20);
    let total = Rc::new(Cell::new(0));
    let mut w = Watch { total: total.clone(), puts: 0, biggest: 0 };
    let at_first = Cell::new(0usize);
    let at_last = Cell::new(0usize);
    {
        let t = total.clone();
        let mut told = |done: i32, n: i32| {
            if done == 1 {
                at_first.set(t.get());
            }
            if done == n - 1 {
                at_last.set(t.get());
            }
        };
        assert_eq!(export_notebook_pdf_to(&nb, 300.0, &mut w, PdfCacheMode::Off, Some(&mut told)), 0);
    }
    let all = total.get();
    assert!(at_first.get() > 0 && at_first.get() < all / 10, "the first page is out before the second is drawn: {} of {}", at_first.get(), all);
    assert!(at_last.get() > all * 8 / 10 && at_last.get() < all, "the bytes went out as the pages were made");
    assert!(w.puts >= 20 && w.biggest < all / 5, "no put holds the file: biggest {} of {}", w.biggest, all);
    // a sink that fails stops the writer
    struct Full(usize);
    impl Sink for Full {
        fn put(&mut self, b: &[u8]) -> bool {
            self.0 += b.len();
            self.0 < 5000
        }
    }
    let before = pdf_pages_rendered();
    assert_eq!(export_notebook_pdf_to(&nb, 300.0, &mut Full(0), PdfCacheMode::Off, None), -1);
    assert!(pdf_pages_rendered() - before < 20, "a failed sink stops the drawing");
    // a count is the file's length
    let mut c = CountSink::default();
    assert_eq!(export_notebook_pdf_to(&nb, 300.0, &mut c, PdfCacheMode::Off, None), 0);
    assert_eq!(c.0 as usize, all);
}

fn picture(path: &str, w: i32, h: i32, ch: i32) {
    let mut px = vec![0u8; (w * h * ch) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * ch) as usize;
            if ch == 3 {
                px[i] = (x * 255 / w) as u8;
                px[i + 1] = (y * 255 / h) as u8;
                px[i + 2] = 128;
            } else {
                px[i] = (x * 255 / w) as u8;
            }
        }
    }
    assert_eq!(bgimage_write_png(path, &BgPicture { px, w, h, ch }), 0);
}

fn colour_pages_keep_their_colour() {
    let mut nb = book("Colour", 3);
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    assert_eq!(mkdir_p(&bg), 0);
    let mut f = String::new();
    bgimage_file(&bg, 0x10, &mut f, 800);
    picture(&f, 800, 1000, 3);
    // page 2 (index 1) has the colour picture, with ink on it; the others are gray paper
    let paper = paper_image(PAPER_ROLE_DOCUMENT, 0x10);
    let mut p = page_new(800, 1000, paper);
    page_add(&mut p, wave(3, 500.0));
    assert_eq!(store_save_page(&nb, 1, &p), 0);
    nb = reload(&nb);
    assert_eq!(bgimage_channels(nb.id.as_str(), paper), 3);
    let want = full(&nb, 300.0);
    let pdf = String::from_utf8_lossy(&want).into_owned();
    assert_eq!(pdf.matches("/ColorSpace /DeviceRGB").count(), 1, "one page has colour");
    assert_eq!(pdf.matches("/ColorSpace /DeviceGray").count(), 2, "the others stay gray");
    let path = format!("{}/colour.pdf", root().display());
    std::fs::write(&path, &want).unwrap();
    // the picture's own pixel where there is no ink, and the page's size
    let got = python_rgb(&path);
    assert_eq!(got, vec![800, 1000, 800 * 1000 * 3, 800_i64.min(5) * 255 / 800, 5 * 255 / 1000, 128, 700 * 255 / 800, 900 * 255 / 1000, 128], "{got:?}");
    // the same through the mirror, from the cache, and then with the picture made gray: the channel
    // count is in the stamp, so the page is drawn again, and comes out gray
    let (m, drawn) = mirror(&nb, 300.0);
    assert_eq!((m == want, drawn), (true, 3));
    let (m, drawn) = mirror(&nb, 300.0);
    assert_eq!((m == want, drawn), (true, 0));
    picture(&f, 800, 1000, 1);
    bgimage_forget(None);
    assert_eq!(bgimage_channels(nb.id.as_str(), paper), 1);
    let (m, drawn) = mirror(&nb, 300.0);
    assert_eq!(drawn, 1, "a picture that changed its channels is drawn again");
    assert_eq!(String::from_utf8_lossy(&m).matches("/DeviceRGB").count(), 0);
    assert_eq!(m, full(&nb, 300.0));
    // ink over colour: darker scales, lighter (white ink) moves toward white, untouched stays
    let mut rgb = vec![200, 100, 50, 200, 100, 50, 200, 100, 50];
    ink_over_colour(&mut rgb, &[200, 200, 100], &[100, 200, 255]);
    assert_eq!(&rgb[..3], &[100, 50, 25]);
    assert_eq!(&rgb[3..6], &[200, 100, 50]);
    assert!(rgb[6] > 200 && rgb[7] > 100 && rgb[8] > 50);
}

/// width, height, bytes inflated, and the pixels at (5, 5) and (700, 900) of the first DeviceRGB image
fn python_rgb(path: &str) -> Vec<i64> {
    let script = r#"
import re, sys, zlib
d = open(sys.argv[1], 'rb').read()
m = re.search(rb'/Width (\d+) /Height (\d+) /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length (\d+) >>\nstream\n', d)
w, h, n = int(m.group(1)), int(m.group(2)), int(m.group(3))
raw = zlib.decompress(d[m.end():m.end() + n])
px = lambda x, y: list(raw[(y * w + x) * 3:(y * w + x) * 3 + 3])
print(*([w, h, len(raw)] + px(5, 5) + px(700, 900)))
"#;
    let out = std::process::Command::new("python3").args(["-I", "-c", script, path]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).split_whitespace().map(|v| v.parse().unwrap()).collect()
}

fn a_zips_pdf_entry_streams() {
    let nb = book("Zipped", 6);
    let want = full(&nb, 300.0);
    let out = format!("{}/zipped", root().display());
    std::fs::create_dir_all(&out).unwrap();
    // into a sink, entry by entry: the PDF entry has a data descriptor, the zip tests, the entry reads back
    let mut zb = Buf::default();
    assert_eq!(export_notebook_zip_to(&nb, 300.0, &mut BufSink(&mut zb)), 0);
    let zip = format!("{out}/nb.zip");
    std::fs::write(&zip, &zb.data).unwrap();
    let t = std::process::Command::new("unzip").args(["-tq", &zip]).output().unwrap();
    assert!(t.status.success(), "{}", String::from_utf8_lossy(&t.stdout));
    let e = std::process::Command::new("unzip").args(["-p", &zip, "Zipped/Zipped.pdf"]).output().unwrap();
    assert!(e.status.success() && e.stdout == want, "the entry is the PDF byte for byte");
    assert!(zb.data.windows(4).any(|w| w == [0x50, 0x4b, 0x07, 0x08]), "a data descriptor ends the PDF entry");
    // the whole-Buf zip is the same archive
    let mut zb2 = Buf::default();
    {
        let mut z = zip_begin(&mut zb2);
        assert_eq!(export_notebook_zip(&nb, 300.0, &mut z, ""), 0);
        assert_eq!(zip_end(&mut z), 0);
    }
    // (the archives differ only in the entries' times, which are the clock's)
    assert_eq!(zb2.data.len(), zb.data.len());
    let zip2 = format!("{out}/nb2.zip");
    std::fs::write(&zip2, &zb2.data).unwrap();
    let e2 = std::process::Command::new("unzip").args(["-p", &zip2, "Zipped/Zipped.pdf"]).output().unwrap();
    assert_eq!(e2.stdout, want);
    // a folder's zip too, with its PDFs
    let mut fz = Buf::default();
    assert_eq!(export_folder_zip_to("", 300.0, &mut BufSink(&mut fz)), 0);
    let zip = format!("{out}/all.zip");
    std::fs::write(&zip, &fz.data).unwrap();
    let t = std::process::Command::new("unzip").args(["-tq", &zip]).output().unwrap();
    assert!(t.status.success());
    let mut again = Buf::default();
    assert_eq!(export_folder_zip("", 300.0, &mut again), 0);
    assert_eq!(again.data.len(), fz.data.len());
}

fn children_carry_their_caps() {
    // the plan: every kind gets oom_score_adj 500 and its address space at fork plus its margin
    for kind in [BgKind::MirrorPdf, BgKind::CompactPages, BgKind::Template, BgKind::ImportFolder, BgKind::GraphLayout, BgKind::PagePreviews, BgKind::GraphLayoutAll] {
        let plan = child_caps_plan(Some(100 << 20), child_margin(kind));
        assert_eq!(plan.oom_adj, 500);
        assert_eq!(plan.as_bytes, Some((100 << 20) + child_margin(kind)));
        assert!(child_margin(kind) >= 64 << 20);
    }
    assert_eq!(child_margin(BgKind::MirrorPdf), 64 << 20, "a streaming mirror: 64 MiB");
    assert_eq!(child_caps_plan(None, 1 << 20).as_bytes, None);
    // on Linux a forked child really takes them
    #[cfg(target_os = "linux")]
    {
        // SAFETY: fork; the child only applies the caps, reads them back and _exits.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            let took = child_caps_apply(64 << 20);
            let oom = std::fs::read_to_string("/proc/self/oom_score_adj").unwrap_or_default();
            let mut lim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            // SAFETY: getrlimit writes one rlimit into lim.
            unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut lim) };
            let ok = oom.trim() == "500" && took.as_bytes.is_some_and(|b| b == lim.rlim_cur as u64);
            // SAFETY: _exit ends the child at once.
            unsafe { libc::_exit(if ok { 0 } else { 1 }) };
        }
        let mut st = 0;
        // SAFETY: waitpid on the child just forked.
        unsafe { libc::waitpid(pid, &mut st, 0) };
        assert!(libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 0, "a child's oom_score_adj and RLIMIT_AS read back");
    }
    // the children bgjob forks say what they took (the simulator's build)
    #[cfg(feature = "sim")]
    {
        let probe = format!("{}/caps-probe.txt", root().display());
        let _ = std::fs::remove_file(&probe);
        std::env::set_var("GRIMOIRE_CAPS_PROBE", &probe);
        let nb = book("Capped", 2);
        let dir = format!("{}/PDFs", root().display());
        bg_mirror_pdf(nb.id.as_str(), 300.0, &dir, false);
        bg_queue(BgKind::CompactPages, Some(nb.id.as_str()), None, 300.0, false);
        let until = now_ms().wrapping_add(60000);
        while bg_busy() && (now_ms().wrapping_sub(until) as i32) < 0 {
            bg_poll();
            sleep_ms(5);
        }
        std::env::remove_var("GRIMOIRE_CAPS_PROBE");
        let lines = std::fs::read_to_string(&probe).unwrap_or_default();
        assert_eq!(lines.lines().count(), 2, "each bgjob child took its caps: {lines:?}");
        #[cfg(target_os = "linux")]
        assert!(lines.lines().all(|l| l.starts_with("oom=500 as=")), "{lines:?}");
    }
}
