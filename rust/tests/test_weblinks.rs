//! weblinks.ini (2.2, issue #174; docs/FORMAT.md, "Web links: weblinks.ini"): the file through the store, what the
//! readers before 2.2 see of a notebook that has one, carrying through Copy page and Copy to notebook, and raw zips.
//! One test function: the store root is process-wide.

use grimoire::prelude::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("weblinks-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn book(name: &str, pages: usize) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_NARROW_NM, &mut nb), 0);
    for i in 1..pages {
        assert_eq!(store_add_page(&mut nb, i as i32, PAPER_NARROW_NM), 0);
    }
    for i in 0..pages {
        let p = page_new(800, 1000, PAPER_NARROW_NM);
        assert_eq!(store_save_page(&nb, i as i32, &p), 0);
    }
    nb
}

fn dir(nb: &NotebookInfo) -> PathBuf {
    root().join("notebooks").join(nb.id.as_str())
}

fn wrec(src: u32, x: i32, index: u32, address: &str) -> WebRecord {
    WebRecord { src, rect: Rect::new(x, 20, 120, 40), index, address: address.to_string() }
}

fn drec(src: u32, dst: u32, x: i32, index: u32) -> DocRecord {
    DocRecord { src, dst, rect: Rect::new(x, 600, 100, 100), quad: None, index }
}

#[test]
fn web_links_file_older_readers_carrying_and_zips() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, 800, 1000);
    saved_loaded_and_never_made_for_none();
    a_file_that_does_not_read_is_left_alone();
    what_a_reader_before_two_two_sees();
    copy_page_and_copy_to_notebook_carry_them();
    raw_zips_carry_the_file();
}

fn saved_loaded_and_never_made_for_none() {
    let nb = book("Web saved", 3);
    let ids = nb.page_ids.clone();
    // no link: no file, and the save says it is done
    let mut wl = WebLinks::default();
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), 0);
    assert!(!dir(&nb).join("weblinks.ini").exists(), "a notebook without web links has no weblinks.ini");
    let exists = |p: u32| ids.contains(&p);
    let st = weblinks_fill(&mut wl, &[wrec(ids[0], 10, 0, "https://a.example/x"), wrec(ids[2], 10, 0, "http://b.example")], 800, 1000, &exists);
    assert_eq!(st.added, 2);
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), 0);
    let back = weblinks_load(nb.id.as_str());
    assert!(back.writable && back.file);
    assert_eq!(back.links, wl.links);
    // a page gone from the notebook: its links are not loaded into the editor's view, and the next write drops them
    let mut nb2 = nb.clone();
    nb2.page_ids.retain(|&p| p != ids[2]);
    let mut wl2 = weblinks_load(nb.id.as_str());
    assert_eq!(weblinks_prune(&mut wl2, &nb2), 1);
    let mut wl3 = weblinks_load(nb.id.as_str());
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl3, &nb2), 0);
    assert_eq!(weblinks_load(nb.id.as_str()).links.len(), 1);
    // a link through the file is shown exactly as stored
    let text = std::fs::read_to_string(dir(&nb).join("weblinks.ini")).unwrap();
    assert!(text.lines().any(|l| l.ends_with(",https://a.example/x")), "{text}");
}

fn a_file_that_does_not_read_is_left_alone() {
    let nb = book("Web bad", 2);
    let path = dir(&nb).join("weblinks.ini");
    for bad in [b"web.1=1,0,0,5,5,https://a\n\0tail".to_vec(), format!("x={}\n", "y".repeat(300)).into_bytes()] {
        std::fs::write(&path, &bad).unwrap();
        let mut wl = weblinks_load(nb.id.as_str());
        assert!(!wl.writable && wl.links.is_empty());
        assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), -1);
        assert_eq!(weblinks_copy_page(&mut wl, nb.page_ids[0], 77), -1);
        assert_eq!(std::fs::read(&path).unwrap(), bad, "never written over");
    }
    // a link at the name is not followed: it reads as a file that does not read
    std::fs::remove_file(&path).unwrap();
    let target = root().join("elsewhere.ini");
    std::fs::write(&target, b"web.1=1,0,0,5,5,https://a\n").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    let mut wl = weblinks_load(nb.id.as_str());
    assert!(!wl.writable);
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), -1);
    assert_eq!(std::fs::read(&target).unwrap(), b"web.1=1,0,0,5,5,https://a\n");
}

// What a version before 2.2 (2.1.0's doclinks.rs and nav.rs readers, which this version's are: doclinks.rs is the
// same code, and nav.rs differs only in how a sidecar is opened) sees of a notebook with web links: its document
// links and its own links, every one of them, and nothing carried (the web links are in a file it never opens);
// and what it writes leaves weblinks.ini as it was.
fn what_a_reader_before_two_two_sees() {
    let nb = book("Web older", 3);
    let ids = nb.page_ids.clone();
    let exists = |p: u32| ids.contains(&p);
    let mut dl = DocLinks::default();
    pdflinks_fill(&mut dl, &[drec(ids[0], ids[1], 10, 0), drec(ids[1], ids[2], 10, 0)], 800, 1000, &exists, &|_, _| false);
    assert_eq!(doclinks_save(nb.id.as_str(), &mut dl, &nb), 0);
    let doc_before = std::fs::read(dir(&nb).join("doclinks.ini")).unwrap();
    let mut wl = WebLinks::default();
    let many: Vec<WebRecord> = (0..600u32).map(|k| wrec(ids[(k % 3) as usize], (k % 6) as i32 * 100, k, &format!("https://example.org/{k}/{}", "p".repeat(100)))).collect();
    weblinks_fill(&mut wl, &many, 800, 1000, &exists);
    assert_eq!(wl.links.len(), 600);
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), 0);
    let web_before = std::fs::read(dir(&nb).join("weblinks.ini")).unwrap();
    assert!(web_before.len() > 16 * 1024, "more than an older doclinks.ini could carry");
    // doclinks.ini is what it was, byte for byte, and reads whole, carrying nothing
    assert_eq!(std::fs::read(dir(&nb).join("doclinks.ini")).unwrap(), doc_before);
    let old = doclinks_load(nb.id.as_str());
    assert!(old.writable);
    assert_eq!(old.links.len(), 2);
    assert!(old.unknown.is_empty());
    // a write of doclinks.ini (a Copy page in 2.1.0) and of links.ini leaves weblinks.ini alone
    let mut old = doclinks_load(nb.id.as_str());
    assert_eq!(doclinks_copy_page(&mut old, ids[0], 4242), 1);
    let mut nb2 = nb.clone();
    nb2.page_ids.push(4242);
    assert_eq!(doclinks_save(nb.id.as_str(), &mut old, &nb2), 0);
    let mut nav = Nav::default();
    nav_init(&mut nav);
    assert_eq!(nav_load(nb.id.as_str(), &mut nav), 0);
    assert_eq!(nav_link_set(&mut nav, ids[2], 5, &format!("p{}", ids[0])), 0);
    assert_eq!(nav_link_set_area(&mut nav, ids[2], 5, Some(Rect::new(1, 1, 50, 50))), 0);
    assert_eq!(nav_save_links(nb.id.as_str(), &mut nav, &nb), 0);
    nav_free(&mut nav);
    assert_eq!(std::fs::read(dir(&nb).join("weblinks.ini")).unwrap(), web_before);
    // and this version reads them all again
    assert_eq!(weblinks_load(nb.id.as_str()).links.len(), 600);
}

fn copy_page_and_copy_to_notebook_carry_them() {
    let nb = book("Web carry src", 3);
    let ids = nb.page_ids.clone();
    let exists = |p: u32| ids.contains(&p);
    let mut wl = WebLinks::default();
    weblinks_fill(&mut wl, &[wrec(ids[0], 10, 0, "https://a.example"), wrec(ids[0], 300, 1, "https://b.example"), wrec(ids[1], 10, 0, "https://c.example")], 800, 1000, &exists);
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), 0);
    // Copy page: the copy gets its page's links, seeds, areas and addresses as they were
    let mut work = weblinks_load(nb.id.as_str());
    let mut nb2 = nb.clone();
    nb2.page_ids.insert(1, 999);
    assert_eq!(weblinks_copy_page(&mut work, ids[0], 999), 2);
    assert_eq!(weblinks_save(nb.id.as_str(), &mut work, &nb2), 0);
    let got = weblinks_load(nb.id.as_str());
    let a: Vec<(u32, Rect, String)> = weblinks_of(&got, ids[0]).iter().map(|l| (l.seed, l.area, l.address.clone())).collect();
    let b: Vec<(u32, Rect, String)> = weblinks_of(&got, 999).iter().map(|l| (l.seed, l.area, l.address.clone())).collect();
    assert_eq!(a, b);
    // Copy to notebook: a web link goes wherever its page goes (it points nowhere in the notebook), scaled
    let dst = book("Web carry dst", 1);
    let copies = [PageCopy { from: ids[0], to: dst.page_ids[0], erased: false }];
    assert_eq!(weblinks_copy_across(&wl, &copies, &dst, 2.0, 2.0), 2);
    let there = weblinks_load(dst.id.as_str());
    assert_eq!(weblinks_of(&there, dst.page_ids[0])[0].area, Rect::new(20, 40, 240, 80));
    assert_eq!(weblinks_of(&there, dst.page_ids[0])[0].address, "https://a.example");
    // nothing to carry: no file made
    let none = book("Web carry none", 1);
    let copies = [PageCopy { from: ids[2], to: none.page_ids[0], erased: false }];
    assert_eq!(weblinks_copy_across(&wl, &copies, &none, 1.0, 1.0), 0);
    assert!(!dir(&none).join("weblinks.ini").exists());
    // a destination whose file does not read gets the pages without them, and the file is left alone
    let bad = book("Web carry bad", 1);
    std::fs::write(dir(&bad).join("weblinks.ini"), b"\0").unwrap();
    let copies = [PageCopy { from: ids[0], to: bad.page_ids[0], erased: false }];
    assert_eq!(weblinks_copy_across(&wl, &copies, &bad, 1.0, 1.0), -1);
    assert_eq!(std::fs::read(dir(&bad).join("weblinks.ini")).unwrap(), b"\0");
    // past the notebook's cap: none copied
    let mut full = WebLinks::default();
    for k in 0..WEBLINKS_MAX as u32 {
        assert!(full.push(WebLink { pid: 5000 + k / 200, seed: k, area: Rect::new(0, 0, 5, 5), address: "https://e.org".into() }));
    }
    assert_eq!(weblinks_copy_page(&mut full, 5000, 6), -1);
    assert!(weblinks_of(&full, 6).is_empty());
}

fn raw_zips_carry_the_file() {
    let nb = book("Web zip", 1);
    let mut wl = WebLinks::default();
    let ids = nb.page_ids.clone();
    weblinks_fill(&mut wl, &[wrec(ids[0], 10, 0, "https://zip.example")], 800, 1000, &|p| ids.contains(&p));
    assert_eq!(weblinks_save(nb.id.as_str(), &mut wl, &nb), 0);
    let mut zout = Buf::default();
    {
        let mut z = zip_begin(&mut zout);
        assert_eq!(export_notebook_zip(&nb, 72.0, &mut z, ""), 0);
    }
    assert!(zout.data.windows(16).any(|w| w == b"raw/weblinks.ini"), "raw zips carry weblinks.ini");
    // the notebook PDF has no web link: it is not written there (DESIGN.md, Web links)
    let mut buf = Buf::default();
    assert_eq!(export_notebook_pdf(&nb, 72.0, &mut buf), 0);
    assert!(!String::from_utf8_lossy(&buf.data).contains("/URI"));
}
