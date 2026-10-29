//! links.ini v2 is a sidecar extension; unreadable input must never be replaced.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn area(x: i32, y: i32, w: i32, h: i32) -> Rect { Rect { x, y, w, h } }

fn roundtrip_and_carry(nb: &NotebookInfo, path: &str) {
    let pid = nb.page_ids[0];
    let bad = ["-1,0,5,5", "0,0,0,5", "5000,0,1001,5", "1,2,3", "1,2,3,4,5", "00000,2,3,4", "1, 2,3,4"];
    let mut text = cformat!("area.%u=BEEF,1,2,3,4\nlink.%u=beef,index\nlink.%u=2,future/path\narea.%u=beef,10,20,30,40\narea.4294967295=1,0,0,1,1\n", pid, pid, pid, pid);
    for b in bad { text.push_str(&cformat!("area.%u=beef,%s\n", pid, b)); }
    text.push_str(&cformat!("area.%u=zz,1,2,3,4\n", pid));
    check!(write_file_atomic(path, text.as_bytes()) == 0);
    let mut n = Nav::default();
    check!(nav_load(nb.id.as_str(), &mut n) == 0);
    let l = nav_link_get(&n, pid, 0xbeef).unwrap();
    check!(l.has_area && l.area == area(10, 20, 30, 40));
    check!(!nav_link_get(&n, pid, 2).unwrap().has_area && n.areas_dropped == 1);
    check!(nav_save_links(nb.id.as_str(), &mut n, nb) == 0 && n.areas_dropped == 0);
    let saved = read_file(path).unwrap();
    let pair = cformat!("link.%u=0000beef,index\narea.%u=0000beef,10,20,30,40\n", pid, pid);
    check!(saved.windows(pair.len()).any(|w| w == pair.as_bytes()));
    check!(!String::from_utf8_lossy(&saved).contains("area.4294967295="));
    for b in bad {
        let bad_line = cformat!("area.%u=beef,%s\n", pid, b);
        check!(saved.windows(bad_line.len()).any(|w| w == bad_line.as_bytes()));
    }
    let records = n.links.clone();
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.links == records);
    check!(nav_link_set(&mut n, pid, l.seed, "p1") == 0 && nav_link_get(&n, pid, l.seed).unwrap().area == l.area);
    check!(nav_link_set_area(&mut n, pid, l.seed, Some(area(i32::MAX, 0, i32::MAX, 1))) == -1);
    check!(nav_link_set_area(&mut n, pid, l.seed, Some(area(0, 0, 6000, 6000))) == 0);
    check!(nav_link_set_area(&mut n, pid, 999, None) == -1);
    nav_link_remove(&mut n, pid, l.seed);
    check!(nav_link_get(&n, pid, l.seed).is_none());
    check!(nav_link_put(&mut n, &l) == 0 && nav_link_get(&n, pid, l.seed) == Some(l));
    let future = records.iter().find(|l| l.seed == 2).unwrap();
    nav_link_remove(&mut n, pid, 2);
    check!(nav_link_put(&mut n, future) == 0 && nav_link_get(&n, pid, 2) == Some(*future));
    let tmp = cformat!("%s.tmp", path);
    check!(std::fs::create_dir(&tmp).is_ok());
    n.areas_dropped = 3;
    check!(nav_save_links(nb.id.as_str(), &mut n, nb) == -1 && n.areas_dropped == 3);
    check!(read_file(path).as_deref() == Some(saved.as_slice()));
    check!(std::fs::remove_dir(tmp).is_ok());
}

fn refuses(nb: &NotebookInfo, path: &str, bytes: &[u8]) {
    check!(write_file_atomic(path, bytes) == 0);
    let mut n = Nav::default();
    check!(nav_load(nb.id.as_str(), &mut n) == -1 && !n.links_writable && n.links.is_empty());
    check!(nav_link_set(&mut n, nb.page_ids[0], 1, "index") == -1);
    check!(nav_save_links(nb.id.as_str(), &mut n, nb) == -1);
    check!(read_file(path).as_deref() == Some(bytes));
}

fn caps_and_protected_files(nb: &NotebookInfo, path: &str) {
    refuses(nb, path, b"link.1=1,index\0tail");
    refuses(nb, path, &vec![b'#'; NAV_LINKS_FILE_MAX + 1]);
    let mut many = String::new();
    for _ in 0..=NAV_LINKS_MAX { many.push_str("area.1=1,0,0,1,1\n"); }
    refuses(nb, path, many.as_bytes());
    let mut order = nb.clone();
    // A notebook holds at most STORE_MAX_PAGES pages: NAV_LINKS_MAX links fill them two by two.
    let pages = NAV_LINKS_MAX as u32 / 2;
    check!(pages as usize <= STORE_MAX_PAGES);
    order.page_ids = (1..=pages).collect();
    let mut n = Nav::default();
    nav_init(&mut n);
    for pid in 1..=pages {
        for seed in [u32::MAX - 1, u32::MAX] {
            let l = NavLink { pid, seed, target: FixedStr::from_str("future-target"), area: area(5000, 5000, 1000, 1000), has_area: true };
            check!(nav_link_put(&mut n, &l) == 0);
        }
    }
    check!(nav_save_links(nb.id.as_str(), &mut n, &order) == 0);
    let bytes = read_file(path).unwrap();
    check!(bytes.len() > NAV_FILE_MAX && bytes.len() <= NAV_LINKS_FILE_MAX);
    let expected = n.links.clone();
    check!(nav_load(nb.id.as_str(), &mut n) == 0 && n.links == expected);
    check!(std::fs::remove_file(path).is_ok());
    check!(std::os::unix::fs::symlink("absent-links", path).is_ok());
    check!(nav_load(nb.id.as_str(), &mut n) == -1 && !n.links_writable);
    check!(nav_save_links(nb.id.as_str(), &mut n, nb) == -1);
    check!(std::fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    check!(std::fs::remove_file(path).is_ok());
    check!(std::fs::create_dir(path).is_ok());
    check!(nav_load(nb.id.as_str(), &mut n) == -1 && !n.links_writable);
    check!(nav_save_links(nb.id.as_str(), &mut n, nb) == -1);
    check!(std::fs::remove_dir(path).is_ok());
}

#[test]
fn test_nav_areas() {
    let root = cformat!("%s/nav-areas-%u", tl_tmp().as_str(), std::process::id());
    let mut nb = NotebookInfo::default();
    check!(store_init(&root) == 0 && store_create("Areas", "", PAPER_BLANK, &mut nb) == 0);
    let mut dir = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut dir, 700) == 0);
    let path = cformat!("%s/links.ini", dir.as_str());
    roundtrip_and_carry(&nb, &path);
    caps_and_protected_files(&nb, &path);
    check!(std::fs::remove_dir_all(&root).is_ok());
    done();
}
