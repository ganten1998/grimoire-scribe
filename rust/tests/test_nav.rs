// test_nav.c: section names (nav.ini) and links (links.ini).
//
// The C's one binary is two #[test]s here, both ignored until store.rs (store_init, store_create,
// store_notebook_dir, store_add_page, store_load_page, store_save_page) and export.rs (the zip and
// the PDF) are translated, as the C main sets up a store first; each makes its own notebook.
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::cell::RefCell;
use testlib::*;

// The C's file-scope nb, path and links_path.
struct Ctx {
    nb: NotebookInfo,
    path: String,
    links_path: String,
}
thread_local! {
    static CTX: RefCell<Option<Ctx>> = const { RefCell::new(None) };
}
fn nb() -> NotebookInfo {
    CTX.with(|c| c.borrow().as_ref().unwrap().nb.clone())
}
fn path() -> String {
    CTX.with(|c| c.borrow().as_ref().unwrap().path.clone())
}
fn links_path() -> String {
    CTX.with(|c| c.borrow().as_ref().unwrap().links_path.clone())
}

// strstr / memmem on bytes
fn has(hay: &[u8], needle: &str) -> bool {
    let n = needle.as_bytes();
    n.is_empty() || hay.windows(n.len()).any(|w| w == n)
}

fn exists(p: &str) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

fn names_follow_page_ids() {
    let nb = nb();
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.writable && n.names.is_empty());
    check!(nav_name(&n, 10).is_empty());
    check!(nav_set_name(&mut n, 40, "  Recipes\n2026  ") == 0);
    check!(nav_set_name(&mut n, 12, "Travel") == 0);
    check!(nav_set_name(&mut n, 0, "Never") != 0);
    check!(nav_name(&n, 40) == "Recipes 2026");
    let mut order = nb.clone();
    order.page_ids = vec![40, 7, 12, 5];
    let mut indices = [-1, -1, -1, -1];
    check!(nav_index(&n, &order, &mut indices[..4]) == 2 && indices[0] == 0 && indices[1] == 2);
    check!(nav_section_of(&n, &order, -1) == -1);
    check!(nav_section_of(&n, &order, 1) == 0 && nav_section_of(&n, &order, 3) == 2);
    order.page_ids[0] = 12;
    order.page_ids[2] = 40; // reorder, without changing either name
    check!(nav_index(&n, &order, &mut indices[..1]) == 2 && indices[0] == 0);
    check!(nav_name(&n, order.page_ids[2]) == "Recipes 2026");
    check!(nav_save(nb.id.as_str(), &mut n, &order) == 0);
    check!(nav_save("other-notebook", &mut n, &order) != 0);
    let mut back = Nav::default();
    nav_init(&mut back);
    check!(nav_load(nb.id.as_str(), &mut back) == 0 && back.names.len() == 2);
    check!(nav_name(&back, 12) == "Travel" && nav_name(&back, 40) == "Recipes 2026");
    check!(nav_set_name(&mut back, 12, "\t \r\n\x7f") == 0 && nav_name(&back, 12).is_empty() && back.names.len() == 1);
    nav_free(&mut back);
    nav_free(&mut n);
}

// (this one needs no store)
fn names_keep_unicode() {
    let mut n = Nav::default();
    nav_init(&mut n);
    let title = "\u{65e5}\u{8a18} \u{1f3b5} = m\u{fa}sica";
    check!(nav_set_name(&mut n, 1, title) == 0 && nav_name(&n, 1) == title);
    let mut big = "a".repeat(92);
    big.push_str("\u{e9}\u{1f3b5} end");
    check!(nav_set_name(&mut n, 2, &big) == 0 && nav_name(&n, 2).len() == 94);
    check!(nav_name(&n, 2)[92..] == *"\u{e9}"); // never half of the four-byte music note
    check!(nav_set_name(&mut n, 2, "\x7f \tNew\tsection\x7f") == 0);
    check!(nav_name(&n, 2) == "New section");
    nav_free(&mut n);
}

fn future_lines_and_last_name_win() {
    let nb = nb();
    let text = "# section names\r\nname.12=First\r\nlink.40=12345678,index\r\n\
                name.40=Recipes=2026\nname.12=Travel\nfuture.target=opaque value\n\
                name.4294967296=Future id\nname.-1=Unknown\nname.12=Last";
    check!(write_file_atomic(&path(), text.as_bytes()) == 0);
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.names.len() == 2);
    check!(nav_name(&n, 12) == "Last" && nav_name(&n, 40) == "Recipes=2026");
    let mut order = nb.clone();
    order.page_ids = vec![12, 40];
    check!(nav_save(nb.id.as_str(), &mut n, &order) == 0);
    let saved = read_file(&path());
    check!(saved.as_ref().is_some_and(|s| has(s, "link.40=12345678,index\n") && has(s, "future.target=opaque value\n")));
    check!(saved.as_ref().is_some_and(|s| has(s, "name.4294967296=Future id\n") && has(s, "name.-1=Unknown\n")));
    nav_free(&mut n);
}

fn failed_save_does_not_prune() {
    let nb = nb();
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.names.len() == 2);
    let mut order = nb.clone();
    order.page_ids = vec![40];
    let tmp = cformat!("%s.tmp", path().as_str());
    let old = read_file(&path());
    check!(std::fs::create_dir(&tmp).is_ok());
    check!(nav_save(nb.id.as_str(), &mut n, &order) != 0 && n.names.len() == 2 && !nav_name(&n, 12).is_empty());
    let saved = read_file(&path());
    check!(saved.is_some() && old.is_some() && saved == old);
    check!(std::fs::remove_dir(&tmp).is_ok());
    check!(nav_save(nb.id.as_str(), &mut n, &order) == 0 && n.names.len() == 1 && nav_name(&n, 12).is_empty());
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.names.len() == 1);
    nav_free(&mut n);
}

fn refuses(bytes: &[u8]) {
    let nb = nb();
    check!(write_file_atomic(&path(), bytes) == 0);
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) != 0 && !n.writable && n.names.is_empty());
    check!(nav_set_name(&mut n, 1, "Must not replace") != 0 && nav_save(nb.id.as_str(), &mut n, &nb) != 0);
    let back = read_file(&path());
    check!(back.as_deref() == Some(bytes));
    nav_free(&mut n);
}

fn unreadable_files_stay_untouched() {
    let nb = nb();
    let binary = b"name.1=Hidden\0tail\0";
    refuses(binary);
    let mut huge = vec![b'x'; NAV_FILE_MAX + 1];
    refuses(&huge);
    huge[..256].fill(b'x');
    huge[256] = b'\n';
    refuses(&huge[..257]); // an unknown line too long to carry safely
    for i in 0..NAV_UNKNOWN_MAX + 2 {
        huge[i] = if (i & 1) != 0 { b'\n' } else { b'x' };
    }
    refuses(&huge[..NAV_UNKNOWN_MAX + 2]);
    check!(std::fs::remove_file(path()).is_ok());
    let cpath = std::ffi::CString::new(path()).unwrap();
    // SAFETY: mkfifo reads the NUL-terminated path we own.
    check!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) } == 0);
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) != 0 && !n.writable); // must not wait for a FIFO writer
    check!(nav_save(nb.id.as_str(), &mut n, &nb) != 0);
    check!(std::fs::remove_file(path()).is_ok());
    check!(std::os::unix::fs::symlink("absent-index", path()).is_ok());
    check!(nav_load(nb.id.as_str(), &mut n) != 0 && !n.writable); // a dangling link is not an absent index
    check!(nav_save(nb.id.as_str(), &mut n, &nb) != 0);
    check!(std::fs::symlink_metadata(path()).is_ok_and(|m| m.file_type().is_symlink()));
    check!(std::fs::remove_file(path()).is_ok());
    nav_free(&mut n);
}

// A stroke through the points given (x, y pairs), with this seed.
fn stroke_at(seed: u32, tool: ToolKind, xy: &[f32]) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    prm.size_min_mm = 0.5;
    prm.size_max_mm = 0.5;
    prm.opacity_min = 1.0;
    prm.opacity_max = 1.0;
    let mut s = stroke_new(tool, &prm, seed);
    for i in 0..xy.len() / 2 {
        let pt = PenSample { x: xy[2 * i], y: xy[2 * i + 1], pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 };
        stroke_append(&mut s, &pt, 300.0);
    }
    s
}

fn links_follow_their_boxes() {
    let nb = nb();
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.links_writable && !n.links_file && n.links.is_empty());
    let mut p = page_new(600, 800, PAPER_BLANK);
    let box_ = [100.0, 100.0, 200.0, 100.0, 200.0, 160.0, 100.0, 160.0, 100.0, 100.0];
    let word = [120.0, 120.0, 180.0, 140.0];
    let dot = [400.0, 400.0];
    page_add(&mut p, stroke_at(0x1234abcd, TOOL_FINELINER, &word));
    page_add(&mut p, stroke_at(0xa0a0a0a0, TOOL_FINELINER, &box_));
    page_add(&mut p, stroke_at(0x0000beef, TOOL_FINELINER, &dot));
    check!(nav_link_set(&mut n, 7, 0xa0a0a0a0, "p9") == 0);
    check!(nav_link_set(&mut n, 7, 0x0000beef, "index") == 0);
    check!(nav_link_set(&mut n, 7, 0x1111, "q9") != 0 && nav_link_set(&mut n, 7, 0x1111, "p0") != 0);
    check!(nav_link_set(&mut n, 7, 0x1111, "p09") != 0 && nav_link_set(&mut n, 0, 0x1111, "p9") != 0);
    check!(nav_link_set(&mut n, 3, 0x1111, "p7") == 0 && n.links.len() == 3);
    let mut first = 0;
    let count = nav_links_of(&n, 7, &mut first);
    check!(count == 2 && n.links[first as usize].seed == 0x0000beef && n.links[first as usize + 1].seed == 0xa0a0a0a0);
    // The box: the box stroke's points, the lone dot made 40 px a side around it.
    let mut got = [NavLink::default(); 4];
    let mut areas = [Rect::default(); 4];
    check!(nav_link_areas(&n, &p, 7, 40.0, &mut got, &mut areas) == 2);
    check!(got[0].seed == 0x0000beef && areas[0].x == 380 && areas[0].y == 380 && areas[0].w == 40 && areas[0].h == 40);
    check!(got[1].seed == 0xa0a0a0a0 && areas[1].x == 100 && areas[1].y == 100 && areas[1].w == 100 && areas[1].h == 60);
    let mut at = NavLink::default();
    check!(nav_link_at(&n, &p, 7, 150.0, 130.0, 40.0, &mut at) && at.seed == 0xa0a0a0a0 && at.target == "p9");
    check!(nav_link_at(&n, &p, 7, 401.0, 399.0, 40.0, &mut at) && at.target == "index");
    check!(!nav_link_at(&n, &p, 7, 300.0, 300.0, 40.0, &mut at));
    check!(!nav_link_at(&n, &p, 3, 150.0, 130.0, 40.0, &mut at)); // another page's link, not this page's
    // The one drawn last wins where boxes overlap; a box with no strokes left does nothing.
    let over = [140.0, 110.0, 260.0, 150.0];
    page_add(&mut p, stroke_at(0x5555, TOOL_FINELINER, &over));
    check!(nav_link_set(&mut n, 7, 0x5555, "p3") == 0);
    check!(nav_link_at(&n, &p, 7, 150.0, 130.0, 40.0, &mut at) && at.seed == 0x5555);
    check!(nav_link_at(&n, &p, 7, 110.0, 150.0, 40.0, &mut at) && at.seed == 0xa0a0a0a0);
    stroke_free(page_remove(&mut p, 3));
    check!(nav_link_at(&n, &p, 7, 150.0, 130.0, 40.0, &mut at) && at.seed == 0xa0a0a0a0);
    check!(nav_link_areas(&n, &p, 7, 40.0, &mut got, &mut areas) == 2 && nav_links_of(&n, 7, &mut first) == 3);
    // An erase stroke with a link's seed is no part of its box.
    page_add(&mut p, stroke_at(0xa0a0a0a0, TOOL_ERASER, &dot));
    check!(nav_link_areas(&n, &p, 7, 40.0, &mut got, &mut areas) == 2 && areas[1].w == 100);
    // New ink never takes a seed a link or an ink stroke has; an erase stroke's seed is free.
    check!(nav_new_seed(&n, &p, 7, 0x1234abcd) != 0x1234abcd);
    check!(nav_new_seed(&n, &p, 7, 0x5555) != 0x5555); // its box is gone, its link is not
    let fresh = nav_new_seed(&n, &p, 7, 0x0000beef);
    check!(fresh != 0x0000beef && fresh != 0xa0a0a0a0 && fresh != 0x1234abcd && fresh != 0x5555);
    check!(nav_new_seed(&n, &p, 7, 0x42) == 0x42 && nav_new_seed(&n, &p, 3, 0x5555) == 0x5555);
    nav_link_remove(&mut n, 7, 0x5555);
    check!(nav_links_of(&n, 7, &mut first) == 2);
    check!(nav_link_set(&mut n, 7, 0xa0a0a0a0, "index") == 0 && nav_links_of(&n, 7, &mut first) == 2);
    // Where they go.
    let mut order = nb.clone();
    order.page_ids = vec![9, 7, 3];
    let link = |target: &str| NavLink { pid: 7, seed: 1, target: FixedStr::from_str(target) };
    let (to9, toindex, gone, future) = (link("p9"), link("index"), link("p44"), link("s2"));
    check!(nav_link_page(&to9, &order) == 0 && nav_link_page(&toindex, &order) == NAV_TO_INDEX);
    check!(nav_link_page(&gone, &order) == -1 && nav_link_page(&future, &order) == -1);
    page_free(p);
    nav_free(&mut n);
}

fn links_save_prune_and_carry() {
    let nb = nb();
    let _ = std::fs::remove_file(links_path());
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_load(nb.id.as_str(), &mut n) == 0);
    let mut order = nb.clone();
    order.page_ids = vec![9, 7];
    check!(nav_save_links(nb.id.as_str(), &mut n, &order) == 0 && !exists(&links_path())); // none: no file
    check!(nav_link_set(&mut n, 7, 0xa0a0a0a0, "p9") == 0 && nav_link_set(&mut n, 7, 0xbeef, "index") == 0);
    check!(nav_link_set(&mut n, 9, 0x10, "p7") == 0 && nav_link_set(&mut n, 12, 0x10, "p7") == 0); // 12: deleted
    check!(nav_link_set(&mut n, 9, 0x20, "p12") == 0); // to a deleted page
    check!(nav_save_links(nb.id.as_str(), &mut n, &order) == 0 && n.links.len() == 3);
    let saved = read_file(&links_path());
    check!(
        saved.as_deref()
            == Some(
                &b"# Grimoire: links drawn on pages\nlink.7=0000beef,index\n\
                   link.7=a0a0a0a0,p9\nlink.9=00000010,p7\n"[..]
            )
    );
    let text = "# links\r\nlink.7=A0A0A0A0,p7\r\nlink.7=BEEF,p9\nlink.9=1,s3/e2\nlink.9=123456789,p7\n\
                link.9=zz,p7\nlink.0=1,p7\nlink.8=1,\nlink.8=2,0123456789abcdef\nfuture=kept\nlink.7=a0a0a0a0,index";
    check!(write_file_atomic(&links_path(), text.as_bytes()) == 0);
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.links.len() == 3 && !n.links_unknown.is_empty());
    let mut first = 0;
    check!(nav_links_of(&n, 7, &mut first) == 2 && n.links[first as usize].target == "p9" && n.links[first as usize + 1].target == "index"); // the last line for a link wins
    check!(nav_links_of(&n, 9, &mut first) == 1 && n.links[first as usize].target == "s3/e2");
    check!(nav_save_links(nb.id.as_str(), &mut n, &order) == 0);
    let saved = read_file(&links_path());
    check!(saved.as_ref().is_some_and(|s| has(s, "link.9=00000001,s3/e2\n") && has(s, "link.9=123456789,p7\n")));
    check!(saved.as_ref().is_some_and(|s| has(s, "link.9=zz,p7\n") && has(s, "link.0=1,p7\n") && has(s, "future=kept\n")));
    check!(saved.as_ref().is_some_and(|s| has(s, "link.8=2,0123456789abcdef\n") && has(s, "link.8=1,\n")));
    // Every link gone: the file stays, empty of them.
    nav_link_remove(&mut n, 7, 0xbeef);
    nav_link_remove(&mut n, 7, 0xa0a0a0a0);
    nav_link_remove(&mut n, 9, 1);
    check!(nav_save_links(nb.id.as_str(), &mut n, &order) == 0 && exists(&links_path()));
    nav_free(&mut n);
    let _ = std::fs::remove_file(links_path());
}

fn links_limits_and_unreadable() {
    let nb = nb();
    let mut n = Nav::default();
    nav_init(&mut n);
    for s in 0..NAV_LINKS_PER_PAGE as u32 {
        check!(nav_link_set(&mut n, 5, s, "index") == 0);
    }
    check!(nav_link_set(&mut n, 5, 9999, "index") != 0); // a full page
    check!(nav_link_set(&mut n, 5, 3, "p8") == 0); // a new target for one it has
    nav_free(&mut n);
    // An unreadable links.ini keeps the names working and is never written over.
    let binary = b"link.1=1,index\0tail\0";
    check!(write_file_atomic(&links_path(), binary) == 0);
    nav_init(&mut n);
    check!(nav_set_name(&mut n, nb.page_ids[0], "Still named") == 0 && nav_save(nb.id.as_str(), &mut n, &nb) == 0);
    check!(nav_load(nb.id.as_str(), &mut n) != 0 && n.writable && !n.links_writable && nav_name(&n, nb.page_ids[0]) == "Still named");
    check!(nav_link_set(&mut n, nb.page_ids[0], 1, "index") != 0 && nav_save_links(nb.id.as_str(), &mut n, &nb) != 0);
    check!(nav_set_name(&mut n, nb.page_ids[0], "Renamed") == 0 && nav_save(nb.id.as_str(), &mut n, &nb) == 0);
    let back = read_file(&links_path());
    check!(back.as_deref() == Some(&binary[..]));
    // And an unreadable nav.ini keeps the links.
    check!(std::fs::remove_file(links_path()).is_ok());
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && nav_link_set(&mut n, nb.page_ids[0], 1, "index") == 0 && nav_save_links(nb.id.as_str(), &mut n, &nb) == 0);
    check!(write_file_atomic(&path(), binary) == 0);
    check!(nav_load(nb.id.as_str(), &mut n) != 0 && !n.writable && n.links_writable && n.links.len() == 1);
    nav_free(&mut n);
    check!(std::fs::remove_file(links_path()).is_ok() && std::fs::remove_file(path()).is_ok());
}

// The two below use export.rs.

fn raw_zip_keeps_index() {
    let nb = nb();
    let mut n = Nav::default();
    nav_init(&mut n);
    check!(nav_set_name(&mut n, nb.page_ids[0], "Notes & recipes") == 0);
    check!(nav_save(nb.id.as_str(), &mut n, &nb) == 0);
    let mut b = Buf::default();
    {
        let mut z = zip_begin(&mut b);
        check!(export_notebook_zip(&nb, 300.0, &mut z, "") == 0 && zip_end(&mut z) == 0);
    }
    let archive = cformat!("%s.zip", path().as_str());
    check!(write_file_atomic(&archive, &b.data) == 0);
    buf_free(&mut b);
    nav_free(&mut n);
    let status = std::process::Command::new("python3")
        .arg("-c")
        .arg(
            "import zipfile,sys; z=zipfile.ZipFile(sys.argv[1]); \
             ns=[n for n in z.namelist() if n.endswith('/raw/nav.ini')]; \
             assert len(ns)==1; assert z.read(ns[0])==open(sys.argv[2],'rb').read()",
        )
        .arg(&archive)
        .arg(path())
        .status();
    check!(status.is_ok_and(|s| s.code() == Some(0)));
}

// Every xref entry of a PDF points at its object: "<n> 0 obj".
fn xref_holds(b: &Buf) -> bool {
    // (as text, with the NULs blanked)
    let text: Vec<u8> = b.data.iter().map(|&c| if c == 0 { b' ' } else { c }).collect();
    let Some(sx) = text.windows(10).position(|w| w == b"startxref\n") else { return false };
    // strtoul(sx + 10, NULL, 10)
    let digits = |from: usize| -> usize {
        let mut v: usize = 0;
        let mut at = from;
        while at < text.len() && text[at].is_ascii_digit() {
            v = v.wrapping_mul(10).wrapping_add((text[at] - b'0') as usize);
            at += 1;
        }
        v
    };
    let at = digits(sx + 10);
    // sscanf(text + at, "xref\n0 %d", &count)
    if at >= text.len() || !text[at..].starts_with(b"xref\n0 ") {
        return false;
    }
    let count = digits(at + 7) as i32;
    if count <= 1 {
        return false;
    }
    let nl = |from: usize| text[from..].iter().position(|&c| c == b'\n').map(|k| from + k);
    // past the two header lines and the free entry
    let Some(l1) = nl(at) else { return false };
    let Some(l2) = nl(l1 + 1) else { return false };
    let mut row = l2 + 1 + 20;
    for n in 1..count {
        let obj = digits(row);
        let want = cformat!("%d 0 obj\n", n);
        if !(obj < text.len() && text[obj..].starts_with(want.as_bytes())) {
            return false;
        }
        row += 20;
    }
    true
}

fn pdf_has_the_outline_and_links() {
    let mut book = NotebookInfo::default();
    check!(store_create("Linked", "", PAPER_BLANK, &mut book) == 0);
    check!(store_add_page(&mut book, 1, PAPER_BLANK) == 0 && store_add_page(&mut book, 2, PAPER_BLANK) == 0);
    let mut plain = Buf::default();
    check!(export_notebook_pdf(&book, 300.0, &mut plain) == 0 && xref_holds(&plain));
    check!(!plain.data.is_empty() && !has(&plain.data, "/Outlines") && !has(&plain.data, "/Annots"));
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&book, 0, book.page_w, book.page_h, &mut p) == 0 && p.is_some());
    let Some(mut p) = p else { return };
    let box_ = [300.0, 600.0, 900.0, 600.0, 900.0, 900.0, 300.0, 900.0];
    let dot = [1000.0, 1000.0];
    page_add(&mut p, stroke_at(0x77, TOOL_FINELINER, &box_));
    page_add(&mut p, stroke_at(0x88, TOOL_FINELINER, &dot));
    check!(store_save_page(&book, 0, &p) == 0);
    page_free(p);
    let mut n = Nav::default();
    nav_init(&mut n);
    let to3 = cformat!("p%u", book.page_ids[2]);
    check!(
        nav_load(book.id.as_str(), &mut n) == 0
            && nav_set_name(&mut n, book.page_ids[0], "Start \u{e9}") == 0
            && nav_set_name(&mut n, book.page_ids[2], "End") == 0
            && nav_save(book.id.as_str(), &mut n, &book) == 0
    );
    check!(nav_link_set(&mut n, book.page_ids[0], 0x77, &to3) == 0 && nav_link_set(&mut n, book.page_ids[0], 0x88, "index") == 0);
    check!(nav_save_links(book.id.as_str(), &mut n, &book) == 0);
    nav_free(&mut n);
    let mut b = Buf::default();
    check!(export_notebook_pdf(&book, 300.0, &mut b) == 0 && xref_holds(&b));
    // 3 pages: objects 4 to 12, the outline 13 to 15, the one link to a page 16 (the Index's has none).
    let t: Vec<u8> = b.data.iter().map(|&c| if c == 0 { b' ' } else { c }).collect();
    check!(has(&t, "<< /Type /Catalog /Pages 2 0 R /Outlines 13 0 R /PageMode /UseOutlines >>"));
    check!(has(&t, "/Resources << /XObject << /Im0 6 0 R >> >> /Annots [ 16 0 R ] >>"));
    check!(has(&t, "/Resources << /XObject << /Im0 9 0 R >> >> >>"));
    let s: f32 = 72.0 / 300.0;
    let ph = book.page_h as f32 * s;
    let want = cformat!(
        "16 0 obj\n<< /Type /Annot /Subtype /Link /Rect [%.2f %.2f %.2f %.2f] /Border [0 0 0] /Dest [10 0 R /Fit] >>",
        300.0 * s,
        fmla(-900.0, s, ph),
        900.0 * s,
        fmla(-600.0, s, ph)
    );
    check!(has(&t, &want));
    check!(has(&t, "13 0 obj\n<< /Type /Outlines /First 14 0 R /Last 15 0 R /Count 2 >>"));
    check!(has(&t, "14 0 obj\n<< /Title <FEFF00530074006100720074002000E9> /Parent 13 0 R /Next 15 0 R /Dest [4 0 R /Fit] >>"));
    check!(has(&t, "15 0 obj\n<< /Title <FEFF0045006E0064> /Parent 13 0 R /Prev 14 0 R /Dest [10 0 R /Fit] >>"));
    check!(has(&t, "trailer\n<< /Size 17 "));
    buf_free(&mut b);
    buf_free(&mut plain);
}

// The C main: a store in a directory of its own and a notebook in it.
fn setup() -> String {
    let root = cformat!("%s/nav-%d", tl_tmp().as_str(), std::process::id() as i32);
    let mut nb = NotebookInfo::default();
    check!(store_init(&root) == 0 && store_create("Sections", "", PAPER_MEDIUM, &mut nb) == 0);
    let mut dir = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut dir, 700) == 0);
    let mut path = String::new();
    csnprintf!(&mut path, 800, "%s/nav.ini", &dir);
    let mut links_path = String::new();
    csnprintf!(&mut links_path, 800, "%s/links.ini", &dir);
    CTX.with(|c| *c.borrow_mut() = Some(Ctx { nb, path, links_path }));
    root
}

fn teardown(root: &str) {
    check!(std::fs::remove_dir_all(root).is_ok());
}

// The names and the links: store.rs only.
#[test]
#[ignore = "store.rs: store_init, store_create, store_notebook_dir (wave 2, store)"]
fn test_nav() {
    let root = setup();
    run!(names_follow_page_ids);
    run!(names_keep_unicode);
    run!(future_lines_and_last_name_win);
    run!(failed_save_does_not_prune);
    run!(unreadable_files_stay_untouched);
    run!(links_follow_their_boxes);
    run!(links_save_prune_and_carry);
    run!(links_limits_and_unreadable);
    teardown(&root);
    done();
}

// The C's other two: the zip and the PDF of a notebook with names and links.
#[test]
#[ignore = "store.rs: store_add_page, store_load_page, store_save_page; export.rs: zip, PDF (wave 2, store and export)"]
fn test_nav_export() {
    let root = setup();
    run!(raw_zip_keeps_index);
    run!(pdf_has_the_outline_and_links);
    teardown(&root);
    done();
}

// names_keep_unicode needs no store: it runs everywhere.
#[test]
fn test_nav_names_in_memory() {
    run!(names_keep_unicode);
    done();
}
