// test_pdflinks_import.rs: the links and page names a PDF brings (2.1.0-beta.2, lane L6; DESIGN.md 3.10 and 9
// "L6"; areas/pdf.md 5 and 7). The walk is read through a real worker (a fresh exec of the headless runner, as in
// the app): its records, and what the import writes from them (doclinks.ini, nav.ini). Generated PDFs only
// (tests/pdffix).
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::fsview::StoragePath;
use grimoire::pdfimport::*;
use grimoire::pdfwork::*;
use grimoire::prelude::*;
use grimoire::sandbox::*;
use pdffix::*;
use std::collections::BTreeSet;
use std::sync::Mutex;

const PW: i32 = 600;
const PH: i32 = 800;
static LOCK: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

struct Env {
    root: String,
    _g: std::sync::MutexGuard<'static, ()>,
}

fn setup(name: &str) -> Env {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in [
        "GRIMOIRE_PDF_SLICE_PAGES",
        "GRIMOIRE_PDF_BANDS",
        "GRIMOIRE_PDF_NO_MAP",
        "GRIMOIRE_PDF_FREE_BYTES",
        "GRIMOIRE_PDF_WATCHDOG_MS",
        "GRIMOIRE_PDF_TEST_CRASH_PAGE",
        "GRIMOIRE_PDF_TEST_HANG_PAGE",
        "GRIMOIRE_PDF_TEST_ALLOC_PAGE",
        "GRIMOIRE_PDF_TEST_LINKS",
        "GRIMOIRE_WORK_FAIL_STEP",
        "GRIMOIRE_IMPORT_STABLE",
        "GRIMOIRE_SANDBOX",
        "GRIMOIRE_WORK_LEN",
    ] {
        std::env::remove_var(h);
    }
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let storage = base.join(format!("grimoire-pdflinks-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, PW, PH);
    ui_reset();
    Env { root: root.to_str().unwrap().to_string(), _g: g }
}

impl Env {
    fn drop(&self, name: &str, bytes: &[u8]) -> String {
        std::fs::write(format!("{}/import/{name}", self.root), bytes).unwrap();
        format!("grimoire/import/{name}")
    }
    /// Imports a PDF to its end: the notebook's id.
    fn import(&self, name: &str, bytes: &[u8]) -> String {
        let src = self.drop(name, bytes);
        let mut id = String::new();
        store_make_id(&mut id);
        let mut last = SliceEnd::More;
        for _ in 0..800 {
            last = run_slice(&id, &src);
            if last != SliceEnd::More {
                break;
            }
        }
        assert!(matches!(last, SliceEnd::Done { .. }), "{last:?}");
        id
    }
    /// What the walk of the PDF finds, through a worker, at the Scribe page's size.
    fn walk(&self, bytes: &[u8]) -> Collected {
        let path = format!("{}/import/walk.pdf", self.root);
        std::fs::write(&path, bytes).unwrap();
        let f = std::fs::File::open(&path).unwrap();
        let len = bytes.len() as u64;
        let limits = Limits { as_bytes: worker_as_bytes(len, false), cpu_secs: 120, mapped: false, file_len: len };
        use std::os::fd::AsRawFd;
        let mut s = Session::start(f.as_raw_fd(), len, &limits).expect("the worker starts");
        s.probe().expect("the page count");
        let c = s.links(PW, PH).expect("the links");
        s.end();
        c
    }
}

/// (src, dst) of every link found, in order.
fn pairs(c: &Collected) -> Vec<(u32, u32)> {
    c.links.iter().map(|(l, _)| (l.src, l.dst)).collect()
}

fn plain_pages(n: usize) -> Vec<PageSpec> {
    (0..n).map(|_| PageSpec::new(420.0, 595.0, "")).collect()
}

fn on(p: usize, n: usize, links: Vec<Lnk>) -> Vec<Vec<Lnk>> {
    let mut v = vec![Vec::new(); n];
    v[p - 1] = links;
    v
}

fn r(i: usize) -> [f64; 4] {
    let x = 20.0 + i as f64 * 30.0;
    [x, 100.0, x + 20.0, 120.0]
}

#[test]
fn every_kind_of_destination_and_action_resolves_to_its_page() {
    let e = setup("kinds");
    let links = vec![
        Lnk::new(r(0), Act::Page(2)),                     // explicit
        Lnk::new(r(1), Act::PageNum(2)),                  // by a 0-based number: page 3
        Lnk::new(r(2), Act::NameDest("tree1".into())),    // a name, from the name tree (leaf one): page 4
        Lnk::new(r(3), Act::StrDest("tree2".into())),     // a string, from the name tree (leaf two): page 6
        Lnk::new(r(4), Act::GoToName("dict1".into())),    // GoTo, from the PDF 1.1 /Dests: page 7
        Lnk::new(r(5), Act::StrDest("dict2".into())),     // a string naming a /Dests entry: page 8
        Lnk::new(r(6), Act::GoToPage(5)),                 // GoTo, explicit
        Lnk::new(r(7), Act::Named("NextPage")),           // page 2
        Lnk::new(r(8), Act::Named("LastPage")),           // page 8
        Lnk::new(r(9), Act::Named("PrevPage")),           // none before page 1: left out
        Lnk::new(r(10), Act::Named("FirstPage")),         // its own page: left out
        Lnk::new(r(11), Act::Uri("https://example.org")), // web: counted, left out
        Lnk::new(r(12), Act::Launch("a.exe")),            // other: counted, left out
        Lnk::new(r(13), Act::Missing),                    // no such page: left out
        Lnk::new(r(14), Act::Nothing),                    // no action: left out
        Lnk::new(r(15), Act::PageNum(99)),                // out of range: left out
        Lnk::new(r(16), Act::NameDest("nowhere".into())), // unknown name: left out
    ];
    let extra = Extra {
        tree: vec![("tree1".into(), 4), ("tree2".into(), 6), ("zzz".into(), 2)],
        dests: vec![("dict1".into(), 7), ("dict2".into(), 8)],
        ..Default::default()
    };
    let c = e.walk(&make_linked(&plain_pages(8), &on(1, 8, links), &extra));
    assert_eq!(pairs(&c), vec![(1, 2), (1, 3), (1, 4), (1, 6), (1, 7), (1, 8), (1, 5), (1, 2), (1, 8)]);
    // the places inside the walk's counts: one web link, two other actions, five that go nowhere
    assert_eq!(c.left, LeftOut { uri: 1, other: 2, dead: 5, capped: 0 });
    // the order they were drawn in is their index
    assert_eq!(c.links.iter().map(|(l, _)| l.index).collect::<Vec<_>>(), (0..9).collect::<Vec<u32>>());
}

#[test]
fn a_planner_like_file_gives_each_link_its_area_in_page_pixels() {
    let e = setup("areas");
    // 420 x 595 pt on 600 x 800: the height decides, s = 800 / 595, and the page is centred in the width
    let s = (600.0f64 / 420.0).min(800.0 / 595.0);
    let rw = (420.0 * s) as i32;
    let ox = (600 - rw) / 2;
    let l = Lnk::new([100.0, 300.0, 180.0, 360.0], Act::Page(2));
    let c = e.walk(&make_linked(&plain_pages(2), &on(1, 2, vec![l]), &Extra::default()));
    assert_eq!(c.links.len(), 1);
    let l = c.links[0].0;
    // floor of the minimum, ceiling of the maximum
    let (x0, x1) = (ox as f64 + 100.0 * s, ox as f64 + 180.0 * s);
    let (y0, y1) = ((595.0 - 360.0) * s, (595.0 - 300.0) * s);
    assert_eq!((l.x, l.y), (x0.floor() as i32, y0.floor() as i32));
    assert_eq!((l.x + l.w, l.y + l.h), (x1.ceil() as i32, y1.ceil() as i32));
    assert!(c.links[0].1.is_none(), "a plain rectangle has no polygon");
}

#[test]
fn quads_make_a_link_each_up_to_eight_and_a_slanted_one_a_polygon() {
    let e = setup("quads");
    let axis = |x: f64, y: f64| [x, y + 10.0, x + 40.0, y + 10.0, x, y, x + 40.0, y]; // as Acrobat orders a quad's corners
    let mut two = Lnk::new([20.0, 20.0, 200.0, 60.0], Act::Page(2));
    two.quads = vec![axis(20.0, 20.0), axis(100.0, 40.0)];
    let mut slanted = Lnk::new([20.0, 200.0, 140.0, 280.0], Act::Page(2));
    slanted.quads = vec![[30.0, 200.0, 140.0, 230.0, 120.0, 280.0, 20.0, 250.0]];
    let mut many = Lnk::new([20.0, 300.0, 400.0, 340.0], Act::Page(2));
    many.quads = (0..9).map(|k| axis(20.0 + 40.0 * k as f64, 300.0)).collect();
    let mut hidden = Lnk::new(r(0), Act::Page(2));
    hidden.flags = 2;
    let mut noview = Lnk::new(r(1), Act::Page(2));
    noview.flags = 32;
    let mut printed = Lnk::new(r(2), Act::Page(2));
    printed.flags = 4;
    let empty = Lnk::new([50.0, 50.0, 50.0, 90.0], Act::Page(2)); // no width
    let flipped = Lnk::new([400.0, 560.0, 300.0, 500.0], Act::Page(2)); // corners the other way round: normalised
    let c = e.walk(&make_linked(&plain_pages(2), &on(1, 2, vec![two, slanted, many, hidden, noview, printed, empty, flipped]), &Extra::default()));
    // 2 + 1 + 8 + (the hidden and the unseen are not on the paper) 1 + (the empty one is left out) + 1
    assert_eq!(c.links.len(), 2 + 1 + 8 + 1 + 1);
    assert_eq!(c.links.iter().filter(|(_, q)| q.is_some()).count(), 1, "only the slanted quad is a polygon");
    assert_eq!(c.left.dead, 1, "the empty rectangle");
    let q = c.links.iter().find_map(|(_, q)| *q).unwrap();
    // 16ths of a pixel, four corners in order around the quad
    assert!(q.iter().all(|v| v.abs() < 800 * 16));
    // the flipped rectangle is a link like any other
    let last = c.links.last().unwrap().0;
    assert!(last.w > 100 && last.h > 50);
    // the polygon rides along into the file
    let id = e.import("Quads.pdf", &make_linked(&plain_pages(2), &on(1, 2, vec![{
        let mut s = Lnk::new([20.0, 200.0, 140.0, 280.0], Act::Page(2));
        s.quads = vec![[30.0, 200.0, 140.0, 230.0, 120.0, 280.0, 20.0, 250.0]];
        s
    }]), &Extra::default()));
    let dl = doclinks_load(&id);
    assert_eq!(dl.links.len(), 1);
    assert!(dl.outline_of(&dl.links[0]).is_some(), "the slanted quad is an outline");
}

fn names_of(c: &Collected) -> Vec<(u32, String)> {
    c.names.iter().map(|(p, t)| (*p, String::from_utf8(t.clone()).unwrap())).collect()
}

#[test]
fn the_outline_names_pages_whatever_the_titles_encoding() {
    let e = setup("outline");
    let outline = vec![
        OutItem { title: utf16be("Caf\u{e9} \u{65e5}\u{672c}"), page: 1, kids: vec![OutItem { title: b"\xEF\xBB\xBFUTF8 \xE2\x9C\x93".to_vec(), page: 2, kids: vec![] }] },
        // PDFDocEncoding: 0x80 is a bullet, 0xE9 an e acute, 0x18 a breve; a tab and a newline become spaces
        OutItem { title: b"\x80 Pdf\xE9\x18\tdoc\n".to_vec(), page: 3, kids: vec![] },
        // two items on page 1: the first named it
        OutItem::new("Second on page one", 1),
        // a title of controls only names nothing
        OutItem::new("\u{1}\u{2}", 4),
        // too long: cut at 95 bytes between characters
        OutItem { title: utf16be(&"\u{e9}".repeat(80)), page: 5, kids: vec![] },
    ];
    let c = e.walk(&make_linked(&plain_pages(6), &vec![Vec::new(); 6], &Extra { outline, ..Default::default() }));
    let n = names_of(&c);
    assert_eq!(n[0], (1, "Caf\u{e9} \u{65e5}\u{672c}".to_string()), "UTF-16BE with a BOM");
    assert_eq!(n[1], (2, "UTF8 \u{2713}".to_string()), "a child comes right after its parent: depth first");
    assert_eq!(n[2], (3, "\u{2022} Pdf\u{e9}\u{2d8} doc".to_string()), "PDFDocEncoding, controls as spaces, trimmed");
    assert!(!n.iter().any(|x| x.0 == 4), "controls only: no name");
    let long = n.iter().find(|x| x.0 == 5).unwrap();
    assert_eq!(long.1.len(), 94, "47 two-byte characters, not a byte more than 95");
    assert_eq!(n.len(), 4);
}

#[test]
fn an_outline_that_loops_or_nests_deep_ends() {
    let e = setup("outline-hostile");
    let items = vec![OutItem::new("One", 1), OutItem::new("Two", 2)];
    let c = e.walk(&make_linked(&plain_pages(3), &vec![Vec::new(); 3], &Extra { outline: items, outline_loop: true, ..Default::default() }));
    assert_eq!(names_of(&c), vec![(1, "One".into()), (2, "Two".into())], "the /Next loop is walked once");
    // a chain 40 deep: each item the only child of the one before, item d on page d (of 40)
    let mut item = OutItem::new("Level 40", 40);
    for d in (1..40).rev() {
        item = OutItem { title: format!("Level {d}").into_bytes(), page: d, kids: vec![item] };
    }
    let c = e.walk(&make_linked(&plain_pages(40), &vec![Vec::new(); 40], &Extra { outline: vec![item], ..Default::default() }));
    let n = names_of(&c);
    assert_eq!(n.len(), 32, "the walk goes 32 deep and no further");
    assert_eq!(n.last().unwrap().1, "Level 32");
}

#[test]
fn a_name_tree_that_loops_or_is_huge_ends_and_a_missing_page_is_left_out() {
    let e = setup("tree-hostile");
    let link = |name: &str, i: usize| Lnk::new(r(i), Act::NameDest(name.into()));
    // a leaf whose kids list the root again
    let extra = Extra { tree: vec![("a".into(), 2), ("b".into(), 3)], tree_loop: true, ..Default::default() };
    let c = e.walk(&make_linked(&plain_pages(3), &on(1, 3, vec![link("a", 0), link("b", 1)]), &extra));
    assert_eq!(pairs(&c), vec![(1, 2), (1, 3)]);
    // 200,000 entries: the walk takes 100,000 of them (the second leaf first) and says nothing of the rest
    let extra = Extra { tree: vec![("early".into(), 2)], tree_bulk: 200_000, ..Default::default() };
    let t = std::time::Instant::now();
    let c = e.walk(&make_linked(&plain_pages(3), &on(1, 3, vec![link("bulk0199999", 0), link("bulk0000000", 1)]), &extra));
    assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    assert_eq!(c.links.len() + c.left.dead as usize, 2);
    assert_eq!(c.links.len(), 1, "an entry past the cap resolves to nothing");
}

#[test]
fn what_the_worker_says_is_checked_and_survives_a_trip_through_the_work_folder() {
    let mut c = Collected::default();
    let l = |src, dst| Rec::Link(WireLink { src, dst, x: -5, y: 10, w: 40, h: 20, index: 0 });
    assert!(c.take(l(1, 2), 3).is_ok());
    assert!(c.take(Rec::Quad([0, 0, 160, 0, 160, 160, 0, 160]), 3).is_ok());
    assert!(c.take(Rec::Quad([0; 8]), 3).is_err(), "one quad to a link");
    assert!(c.take(l(1, 4), 3).is_err(), "a page the PDF lacks");
    assert!(c.take(l(0, 2), 3).is_err());
    assert!(c.take(Rec::Link(WireLink { src: 1, dst: 2, x: 999_999, y: 0, w: 1, h: 1, index: 0 }), 3).is_err());
    assert!(c.take(Rec::Name { page: 2, text: "Two".into() }, 3).is_ok());
    assert!(c.take(Rec::Name { page: 2, text: "Again".into() }, 3).is_err(), "a name a page");
    assert!(c.take(Rec::Name { page: 3, text: vec![0xff, 0xfe] }, 3).is_err(), "UTF-8 only");
    assert!(c.take(Rec::Name { page: 3, text: vec![b'x'; 96] }, 3).is_err());
    assert!(c.take(Rec::Left(LeftOut { uri: 2, other: 1, dead: 0, capped: 0 }), 3).is_ok());
    assert!(c.take(Rec::Left(LeftOut::default()), 3).is_err(), "one F");
    assert!(c.take(Rec::Pages(3), 3).is_err());
    let back = Collected::from_parts(c.links_text().as_bytes(), c.names_text().as_bytes(), 3).unwrap();
    assert_eq!(back.links, c.links);
    assert_eq!(back.names, c.names);
    assert_eq!(back.left, c.left);
    assert!(Collected::from_parts(b"L 1 2 3\n", b"", 3).is_none());
    assert!(Collected::from_parts(b"L 1 9 0 0 5 5 0\n", b"", 3).is_none(), "a part is read as strictly as the worker's lines");
}

fn arr(v: &[(u32, u32)]) -> BTreeSet<(u32, u32)> {
    v.iter().copied().collect()
}

#[test]
fn an_import_writes_doclinks_and_names_and_the_work_files_go() {
    let e = setup("write");
    let links = vec![
        Lnk::new(r(0), Act::Page(2)),
        Lnk::new(r(1), Act::NameDest("n".into())),
        Lnk::new(r(2), Act::Uri("http://x")),
        Lnk::new(r(3), Act::Named("LastPage")),
    ];
    let extra = Extra { tree: vec![("n".into(), 3)], outline: vec![OutItem::new("Start", 1), OutItem::new("Middle", 2)], ..Default::default() };
    let id = e.import("Doc.pdf", &make_linked(&plain_pages(3), &on(1, 3, links), &extra));
    let dl = doclinks_load(&id);
    let got: Vec<(u32, u32)> = dl.links.iter().map(|l| (l.pid, l.target)).collect();
    assert_eq!(arr(&got), arr(&[(1, 2), (1, 3), (1, 3)]), "the web link is left out");
    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_load(&id, &mut nav), 0);
    assert_eq!((nav_name(&nav, 1), nav_name(&nav, 2), nav_name(&nav, 3)), ("Start", "Middle", ""));
    nav_free(&mut nav);
    // links.ini is the user's own: the PDF's links never go there
    let dir = format!("{}/notebooks/{id}", e.root);
    assert!(!std::path::Path::new(&format!("{dir}/links.ini")).exists());
    for f in ["links.part", "names.part"] {
        assert!(!std::path::Path::new(&format!("{dir}/{f}")).exists(), "{f} is a work file");
    }
    // the first page's links are seeds the user's own never collide with, and the doc links cap is not links.ini's
    assert!(dl.links.iter().all(|l| l.pid == 1));
}

#[test]
fn the_year_planner_fills_to_its_water_level_and_trims_its_densest_page() {
    let e = setup("year");
    let t = std::time::Instant::now();
    let c = e.walk(&year_planner());
    assert_eq!(c.links.len(), 1000 + 12 * 150 + 367 * 33, "every annotation is a link");
    assert_eq!(c.left, LeftOut::default());
    let id = e.import("Year.pdf", &year_planner());
    let dl = doclinks_load(&id);
    let mut per = std::collections::HashMap::new();
    for l in &dl.links {
        *per.entry(l.pid).or_insert(0usize) += 1;
    }
    // the water level: 90% of the 16,000 cap
    assert_eq!(dl.links.len(), 14_400);
    assert_eq!(per[&1], 489, "the year page, the densest, lost links: it keeps what the others leave");
    for p in 2..=13u32 {
        assert_eq!(per[&p], 150, "month page {p} is whole");
    }
    for p in 14..=380u32 {
        assert_eq!(per[&p], 33, "tab page {p} is whole");
    }
    // each page keeps its first links in the order the PDF drew them: the year page's first 489 cells
    let year: Vec<u32> = doclinks_of(&dl, 1).iter().map(|l| l.target).collect();
    let want: Vec<u32> = (0..489usize).map(|k| (((k % 379) + 1) % 380) as u32 + 1).collect();
    assert_eq!(year, want, "annotation order");
    eprintln!("year planner imported in {:?}", t.elapsed());
    // the file is under both caps, and reads back whole
    let size = std::fs::metadata(format!("{}/notebooks/{id}/doclinks.ini", e.root)).unwrap().len() as usize;
    assert!(size <= DOCLINKS_FILE_MAX * 90 / 100, "{size}");
    // and the import told its end that links were left out
    assert_eq!(finished_trimmed(&id), 1000 - 489);
}

/// How many links the finished import `id` says it left out (what the toast reads).
fn finished_trimmed(id: &str) -> u32 {
    import_result(id).map_or(0, |r| r.1)
}

/// A page with filled boxes at the links' rectangles, so the picture itself says where a link must be.
fn boxed_page(w: f64, h: f64, rotate: i32, crop: Option<[f64; 4]>, boxes: &[[f64; 4]]) -> PageSpec {
    let mut c = String::new();
    for b in boxes {
        c.push_str(&gray_rect(0.0, b[0], b[1], b[2] - b[0], b[3] - b[1]));
    }
    let mut p = PageSpec::new(w, h, &c);
    p.rotate = rotate;
    p.crop = crop;
    p
}

fn page_picture(root: &str, id: &str, page: u32) -> (u32, Vec<u8>) {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_load_info(id, &mut nb), 0);
    let mut pgbox = None;
    assert_eq!(store_load_page(&nb, page as i32 - 1, PW, PH, &mut pgbox), 0);
    let pic = paper_image_id(pgbox.as_ref().unwrap().paper);
    let path = format!("{root}/notebooks/{id}/bg/{pic:06x}.png");
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
    let mut rd = dec.read_info().unwrap();
    assert_eq!(rd.info().color_type, png::ColorType::Grayscale);
    let mut buf = vec![0u8; rd.output_buffer_size().unwrap()];
    let f = rd.next_frame(&mut buf).unwrap();
    buf.truncate(f.buffer_size());
    (f.width, buf)
}

fn expect_areas_on_boxes(name: &str, w: f64, h: f64, rotate: i32, crop: Option<[f64; 4]>) {
    let e = setup(name);
    // boxes far apart, inside the crop box
    let c = crop.unwrap_or([0.0, 0.0, w, h]);
    let boxes: Vec<[f64; 4]> = [(0.2, 0.2), (0.6, 0.25), (0.25, 0.65), (0.65, 0.7)]
        .iter()
        .map(|&(fx, fy)| {
            let (x, y) = (c[0] + (c[2] - c[0]) * fx, c[1] + (c[3] - c[1]) * fy);
            [x, y, x + 50.0, y + 40.0]
        })
        .collect();
    let mut pages = vec![boxed_page(w, h, rotate, crop, &boxes)];
    pages.push(PageSpec::new(w, h, ""));
    let links: Vec<Lnk> = boxes.iter().map(|b| Lnk::new(*b, Act::Page(2))).collect();
    let id = e.import("Boxes.pdf", &make_linked(&pages, &on(1, 2, links), &Extra::default()));
    let dl = doclinks_load(&id);
    assert_eq!(dl.links.len(), 4, "{name}");
    let (pw, pix) = page_picture(&e.root, &id, 1);
    let at = |x: i32, y: i32| pix[(y as usize) * pw as usize + x as usize];
    for l in &dl.links {
        let a = l.area();
        // dark inside (two pixels in from every edge), white just outside (three pixels out)
        for (x, y) in [(a.x + 2, a.y + 2), (a.x + a.w - 3, a.y + a.h - 3), (a.x + a.w / 2, a.y + a.h / 2), (a.x + 2, a.y + a.h - 3), (a.x + a.w - 3, a.y + 2)] {
            assert!(at(x, y) < 80, "{name}: ({x},{y}) in {a:?} is dark, not {}", at(x, y));
        }
        for (x, y) in [(a.x - 3, a.y + a.h / 2), (a.x + a.w + 2, a.y + a.h / 2), (a.x + a.w / 2, a.y - 3), (a.x + a.w / 2, a.y + a.h + 2)] {
            assert!(at(x, y) > 200, "{name}: ({x},{y}) beside {a:?} is white, not {}", at(x, y));
        }
    }
}

#[test]
fn every_area_lies_on_its_drawn_box_for_every_rotation_a_crop_a_landscape_and_an_a4_page() {
    for rot in [0, 90, 180, 270] {
        expect_areas_on_boxes(&format!("rot{rot}"), 420.0, 595.0, rot, None);
    }
    expect_areas_on_boxes("crop", 500.0, 700.0, 0, Some([40.0, 60.0, 440.0, 660.0]));
    expect_areas_on_boxes("crop90", 500.0, 700.0, 90, Some([40.0, 60.0, 440.0, 660.0]));
    expect_areas_on_boxes("landscape", 842.0, 595.0, 0, None);
    expect_areas_on_boxes("a4", 595.0, 842.0, 0, None);
}

#[test]
fn a_notebook_exported_and_imported_again_has_the_same_links_and_names() {
    let e = setup("roundtrip");
    // a notebook of 5 pages (600 x 800 px: the import's own page, so the fit is exact)
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Trip", "", PAPER_NARROW_NM, &mut nb), 0);
    for i in 1..5 {
        assert_eq!(store_add_page(&mut nb, i, PAPER_NARROW_NM), 0);
    }
    for i in 0..5 {
        let p = page_new(PW, PH, PAPER_NARROW_NM);
        assert_eq!(store_save_page(&nb, i, &p), 0);
    }
    let ids = nb.page_ids.clone();
    let rec = |s: usize, d: usize, x, y, w, h, i| DocRecord { src: ids[s], dst: ids[d], rect: Rect::new(x, y, w, h), quad: None, index: i };
    let recs = vec![
        rec(0, 1, 40, 60, 200, 80, 0),
        rec(0, 4, 300, 61, 150, 33, 1),
        rec(0, 2, 17, 500, 11, 301, 2),
        rec(1, 0, 5, 5, 40, 40, 0),
        rec(3, 0, 333, 444, 55, 66, 0),
    ];
    let mut dl = DocLinks::default();
    let st = pdflinks_fill(&mut dl, &recs, PW, PH, &|p| ids.contains(&p), &|_, _| false);
    assert_eq!(st.added, 5);
    assert_eq!(doclinks_save(nb.id.as_str(), &mut dl, &nb), 0);
    let mut nav = Nav::default();
    nav_init(&mut nav);
    nav_set_name(&mut nav, ids[0], "Cover");
    nav_set_name(&mut nav, ids[2], "Caf\u{e9} notes");
    nav_set_name(&mut nav, ids[4], "End");
    assert_eq!(nav_save(nb.id.as_str(), &mut nav, &nb), 0);
    nav_free(&mut nav);
    let mut buf = Buf::default();
    assert_eq!(export_notebook_pdf(&nb, 300.0, &mut buf), 0);
    let id = e.import("Trip.pdf", &buf.data);
    let mut back = NotebookInfo::default();
    assert_eq!(store_load_info(&id, &mut back), 0);
    assert_eq!(back.npages(), 5);
    let was = doclinks_load(nb.id.as_str());
    let now = doclinks_load(&id);
    assert_eq!(now.links.len(), was.links.len());
    let pos = |nb: &NotebookInfo, p: u32| nb.page_ids.iter().position(|&x| x == p).unwrap();
    let key = |nb: &NotebookInfo, dl: &DocLinks| -> Vec<(usize, usize, Rect)> {
        let mut v: Vec<_> = dl.links.iter().map(|l| (pos(nb, l.pid), pos(nb, l.target), l.area())).collect();
        v.sort_by_key(|k| (k.0, k.1, k.2.y, k.2.x));
        v
    };
    let (a, b) = (key(&nb, &was), key(&back, &now));
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((x.0, x.1), (y.0, y.1), "same pages");
        let (p, q) = (x.2, y.2);
        assert!(
            (p.x - q.x).abs() <= 1 && (p.y - q.y).abs() <= 1 && (p.x + p.w - q.x - q.w).abs() <= 1 && (p.y + p.h - q.y - q.h).abs() <= 1,
            "{p:?} came back as {q:?}"
        );
    }
    let mut n2 = Nav::default();
    nav_init(&mut n2);
    assert_eq!(nav_load(&id, &mut n2), 0);
    let names: Vec<String> = back.page_ids.iter().map(|&p| nav_name(&n2, p).to_string()).collect();
    assert_eq!(names, vec!["Cover", "", "Caf\u{e9} notes", "", "End"]);
    nav_free(&mut n2);
    let _ = StoragePath::new(b"");
}

#[test]
fn a_worker_that_dies_or_stalls_in_the_link_walk_costs_the_notebook_its_links_only() {
    for (hook, name) in [("1", "dies"), ("2", "stalls")] {
        let e = setup(&format!("walkdies{hook}"));
        std::env::set_var("GRIMOIRE_PDF_TEST_LINKS", hook);
        std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "1500");
        let links = vec![Lnk::new(r(0), Act::Page(2))];
        let extra = Extra { outline: vec![OutItem::new("Start", 1)], ..Default::default() };
        let id = e.import("Doc.pdf", &make_linked(&plain_pages(3), &on(1, 3, links), &extra));
        let mut nb = NotebookInfo::default();
        assert_eq!(store_load_info(&id, &mut nb), 0, "a worker that {name}: the notebook is made");
        assert_eq!(nb.npages(), 3);
        let dir = format!("{}/notebooks/{id}", e.root);
        assert!(!std::path::Path::new(&format!("{dir}/doclinks.ini")).exists(), "a worker that {name}: no links");
        assert!(!std::path::Path::new(&format!("{dir}/nav.ini")).exists(), "a worker that {name}: no names");
        assert_eq!(import_result(&id).map(|r| r.2), Some(0), "no page is blank");
        std::env::remove_var("GRIMOIRE_PDF_TEST_LINKS");
        std::env::remove_var("GRIMOIRE_PDF_WATCHDOG_MS");
    }
}
