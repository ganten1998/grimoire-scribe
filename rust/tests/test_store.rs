// test_store.c: store + export.
//
// Not ported as the C has it: buffer_failure_stays_failed starts from a Buf whose len is
// SIZE_MAX - 1 to make an append overflow. The Rust Buf holds a Vec, which has no such length to
// set, so the overflow itself cannot be staged; what stays is what the test is about, a failed
// buffer staying failed until buf_free, and a good one after it.
// Index loops are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains)]
mod testlib;
use grimoire::prelude::*;
use std::sync::OnceLock;
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
fn root() -> &'static str {
    ROOT.get().map_or("", |s| s.as_str())
}

fn exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
fn unlink(path: &str) -> bool {
    std::fs::remove_file(path).is_ok()
}
fn mkdir(path: &str) -> bool {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().mode(0o755).create(path).is_ok()
}
fn system(cmd: &str) -> i32 {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .ok()
        .and_then(|s| s.code())
        .unwrap_or(-1)
}
/// strtoul(s, NULL, 10) for the digits that start s
fn strtoul(s: &[u8]) -> usize {
    let mut n = 0usize;
    for &c in s {
        if !c.is_ascii_digit() {
            break;
        }
        n = n * 10 + (c - b'0') as usize;
    }
    n
}
fn find(hay: &[u8], needle: &str) -> Option<usize> {
    hay.windows(needle.len())
        .position(|w| w == needle.as_bytes())
}

fn count_pdf_pages(done: i32, total: i32, count: &mut i32) {
    check!(done == *count && done <= total);
    *count += 1;
}

fn buffer_failure_stays_failed() {
    let mut b = Buf {
        err: true,
        ..Default::default()
    };
    check!(buf_append(&mut b, b"xx") == -1 && b.err);
    check!(b.data.is_empty());
    b.data.clear();
    check!(buf_append(&mut b, b"x") == -1 && b.data.is_empty());
    buf_free(&mut b);
    check!(buf_append(&mut b, b"abc") == 0 && b.data.len() == 3 && !b.err);
    buf_free(&mut b);
}

fn squiggle(t: ToolKind, y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    let mut s = stroke_new(t, &p, 99);
    for i in 0..50 {
        let f = i as f32;
        let q = PenSample {
            x: fmla(f, 20.0, 300.0),
            y: fmla(30.0, (f * 0.4).sin(), y),
            pressure: 0.2 + f / 70.0,
            tilt_x: 10.0,
            tilt_y: -5.0,
            t_ms: i as u32 * 4,
        };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

// A page added after a page of 1.1's document role (an imported page image, 0x51......) gets the
// notebook's paper, as 1.1 does: the + button and the page left when every page is deleted pass the
// page they follow, and 1.1 would draw the other page's picture on the new one. A template (role 0)
// and any other newer paper are kept, as are the known kinds.
fn new_page_after_a_document_page() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Annotated", "", PAPER_NARROW, &mut nb) == 0);
    let doc = PaperKind(0x51000123);
    let tpl = PaperKind(0x50000123);
    check!(paper_is_document(doc) && !paper_is_document(tpl));
    check!(!paper_is_document(PaperKind(0x55000123)) && !paper_is_document(PaperKind(0x53000123)));
    check!(!paper_is_document(PAPER_GRID) && !paper_is_document(music_default()));
    check!(store_new_page_paper(&nb, doc) == PAPER_NARROW);
    check!(
        store_new_page_paper(&nb, tpl) == tpl
            && store_new_page_paper(&nb, PAPER_GRID) == PAPER_GRID
    );
    check!(store_new_page_paper(&nb, PaperKind(0x55000123)) == PaperKind(0x55000123));
    check!(store_add_page(&mut nb, 1, doc) == 0 && store_add_page(&mut nb, 2, tpl) == 0);
    let mut p = None;
    check!(
        store_load_page(&nb, 1, 1860, 2480, &mut p) == 0
            && p.as_ref().is_some_and(|p| p.paper == PAPER_NARROW)
    );
    drop(p.take());
    check!(
        store_load_page(&nb, 2, 1860, 2480, &mut p) == 0
            && p.as_ref().is_some_and(|p| p.paper == tpl)
    );
    drop(p.take());
    let mut odd = nb.clone();
    odd.paper = doc; // a notebook whose own paper is a document page's: Blank, never the picture
    check!(store_new_page_paper(&odd, doc) == PAPER_BLANK);
}

fn notebooks() {
    let mut nb = NotebookInfo::default();
    let mut back = NotebookInfo::default();
    check!(store_create("Lecture notes", "", PAPER_NARROW, &mut nb) == 0);
    check!(store_load_info(nb.id.as_str(), &mut back) == 0);
    check!(back.name == "Lecture notes" && back.paper == PAPER_NARROW && back.npages() == 1);
    check!(store_add_page(&mut nb, 1, PAPER_MUSIC) == 0);
    check!(store_add_page(&mut nb, 0, PAPER_WIDE) == 0);
    check!(nb.npages() == 3 && nb.page_ids[0] == 3 && nb.page_ids[1] == 1 && nb.page_ids[2] == 2);
    let mut p = None;
    check!(
        store_load_page(&nb, 2, 1860, 2480, &mut p) == 0
            && p.as_ref().is_some_and(|p| p.paper == PAPER_MUSIC)
    );
    let mut p = p.unwrap();
    page_add(&mut p, squiggle(TOOL_PENCIL, 500.0));
    page_add(&mut p, squiggle(TOOL_BALLPOINT, 900.0));
    check!(store_save_page(&nb, 2, &p) == 0);
    let mut q = None;
    check!(store_load_page(&nb, 2, 1860, 2480, &mut q) == 0);
    let qp = q.unwrap();
    check!(
        qp.strokes.len() == 2
            && qp.strokes[1].pts.len() == 50
            && qp.strokes[1].tool == TOOL_BALLPOINT
    );
    check_near!(qp.strokes[0].pts[10].x, p.strokes[0].pts[10].x, 1e-4);
    check_near!(
        qp.strokes[0].pts[10].pressure,
        p.strokes[0].pts[10].pressure,
        1e-4
    );
    check_near!(qp.strokes[0].pts[10].tilt_y, -5.0f32, 0.01);
    drop(qp);
    // Scaled load
    let mut q = None;
    check!(store_load_page(&nb, 2, 930, 1240, &mut q) == 0);
    let qp = q.unwrap();
    check!(qp.w == 930 && (qp.strokes[0].pts[10].x - p.strokes[0].pts[10].x * 0.5).abs() < 1e-3);
    drop(qp);
    drop(p);
    check!(store_delete_page(&mut nb, 0) == 0 && nb.npages() == 2);
    check!(store_rename(nb.id.as_str(), "Physics") == 0);
    check!(store_set_folder(nb.id.as_str(), "School") == 0);
    let mut nb2 = NotebookInfo::default();
    check!(store_create("Songs", "", PAPER_MUSIC, &mut nb2) == 0);
    store_add_folder("Empty folder");
    let mut folders: Vec<String> = Vec::new();
    check!(store_list_folders(&mut folders) == 0);
    check!(folders.len() == 2 && folders[0] == "Empty folder" && folders[1] == "School");
    let mut list = Vec::new();
    check!(store_list(&mut list) == 0 && list.len() == 2);
    // Export everything
    let mut b = Buf::default();
    let id = nb.id.as_str().to_string();
    check!(store_load_info(&id, &mut nb) == 0);
    check!(export_notebook_pdf(&nb, 300.0, &mut b) == 0);
    check!(b.data.len() > 1000 && b.data.starts_with(b"%PDF-1.4"));
    let mut with_progress = Buf::default();
    let mut completed = 0;
    check!(
        export_notebook_pdf_progress(
            &nb,
            300.0,
            &mut with_progress,
            Some(&mut |done, total| count_pdf_pages(done, total, &mut completed))
        ) == 0
    );
    check!(completed == nb.npages() + 1);
    check!(with_progress.data == b.data);
    buf_free(&mut with_progress);
    // xref offsets must point at "N 0 obj"
    let x = find(&b.data[b.data.len() - 200..], "startxref").map(|i| b.data.len() - 200 + i);
    check!(x.is_some());
    if let Some(x) = x {
        let xo = strtoul(&b.data[x + 10..]);
        check!(b.data[xo..].starts_with(b"xref"));
        let mut row = xo;
        for _ in 0..3 {
            // past "xref", past "0 N", past the free entry
            row += b.data[row..].iter().position(|&c| c == b'\n').unwrap_or(0) + 1;
        }
        let o1 = strtoul(&b.data[row..]);
        check!(b.data[o1..].starts_with(b"1 0 obj"));
    }
    let out = format!("{}/../out", tl_tmp());
    check!(mkdir_p(&out) == 0); // (other tests make it too; this one must not depend on running after them)
    let path = format!("{out}/sample.pdf");
    write_file_atomic(&path, &b.data);
    buf_free(&mut b);
    check!(export_page_png(&nb, 1, 300.0, &mut b) == 0 && &b.data[1..4] == b"PNG");
    buf_free(&mut b);
    check!(export_folder_zip("", 300.0, &mut b) == 0);
    let path = format!("{out}/all.zip");
    write_file_atomic(&path, &b.data);
    buf_free(&mut b);
    let cmd = format!("unzip -tq '{path}' >/dev/null");
    check!(system(&cmd) == 0);
    {
        let mut z = zip_begin(&mut b);
        check!(export_notebook_zip(&nb, 300.0, &mut z, "") == 0);
        zip_end(&mut z);
    }
    let path = format!("{out}/notebook.zip");
    write_file_atomic(&path, &b.data);
    buf_free(&mut b);
    let cmd = format!(
        "unzip -tq '{path}' >/dev/null && unzip -l '{path}' | grep -q 'Physics/page-002.png'"
    );
    check!(system(&cmd) == 0);
    let docs = format!("{}/PDFs", root());
    check!(export_mirror_pdf(&nb, 300.0, &docs) == 0);
    let path = format!("{docs}/School/Physics.pdf");
    check!(exists(&path));
    // The same export through the background queue: the UI never waits for it.
    unlink(&path);
    bg_mirror_pdf(nb.id.as_str(), 300.0, &docs, true);
    bg_mirror_pdf(nb.id.as_str(), 300.0, &docs, false); // queued behind the running one (it may be stale)
    bg_mirror_pdf(nb.id.as_str(), 300.0, &docs, false); // a duplicate of a queued job merges into it
                                                        // Two exports by children built with the sanitizers and niced: on a loaded machine (several
                                                        // gates at once) that takes far longer than on an idle one, so wait for them by the clock.
    let mut got = 0;
    let until = now_ms().wrapping_add(60000);
    while (bg_busy() || got == 0) && (now_ms().wrapping_sub(until) as i32) < 0 {
        let r = bg_poll();
        if r != 0 {
            got = r;
        }
        sleep_ms(10);
    }
    check!(got == 1 && !bg_busy());
    check!(exists(&path));
    check!(store_delete(nb2.id.as_str()) == 0);
    check!(store_list(&mut list) == 0 && list.len() == 1);
}

fn copy_pages_keeps_bytes_and_order() {
    let mut nb = NotebookInfo::default();
    let mut back = NotebookInfo::default();
    check!(store_create("Copies", "", PAPER_NARROW, &mut nb) == 0);
    let mut p = None;
    check!(store_load_page(&nb, 0, 1860, 2480, &mut p) == 0);
    let mut pg = p.unwrap();
    pg.paper = PaperKind(0x51000123);
    page_add(&mut pg, squiggle(TOOL_PENCIL, 500.0));
    check!(store_save_page(&nb, 0, &pg) == 0);
    drop(pg);
    let mut dir = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut dir, 700) == 0);
    let source = format!("{dir}/p1.qpg");
    let before = read_file(&source);
    check!(before.is_some());
    check!(store_duplicate_page(&mut nb, 0, 1) == 0);
    check!(nb.npages() == 2 && nb.page_ids[0] == 1 && nb.page_ids[1] == 2);
    let dest = format!("{dir}/p2.qpg");
    let after = read_file(&dest);
    check!(before.is_some() && after.is_some() && before == after);
    let mut p = None;
    check!(store_load_page(&nb, 1, 1860, 2480, &mut p) == 0);
    check!(p
        .as_ref()
        .is_some_and(|p| p.strokes.len() == 1 && p.paper == PaperKind(0x51000123)));
    drop(p);
    check!(
        store_load_info(nb.id.as_str(), &mut back) == 0
            && back.npages() == 2
            && back.page_ids[1] == 2
    );

    // A page from a future header version is read-only here. Copy retains its original bytes.
    let mut before = before.unwrap_or_default();
    let n = before.len();
    check!(n > 6);
    if n > 6 {
        before[4] = 3;
        before[5] = 0;
        check!(write_file_atomic(&source, &before) == 0);
        check!(store_duplicate_page(&mut nb, 0, 1) == 0);
        let dest = format!("{dir}/p3.qpg");
        let after = read_file(&dest);
        check!(after.as_ref().is_some_and(|a| a == &before));
        let mut p = None;
        check!(
            store_load_page(&nb, 1, 1860, 2480, &mut p) == 0
                && p.as_ref().is_some_and(|p| p.readonly)
        );
    }
    check!(nb.npages() == 3 && nb.page_ids[0] == 1 && nb.page_ids[1] == 3 && nb.page_ids[2] == 2);

    // Only a genuinely missing file is a blank page. It copies the notebook's paper.
    check!(unlink(&source));
    let end = nb.npages();
    check!(store_duplicate_page(&mut nb, 0, end) == 0);
    let mut p = None;
    check!(store_load_page(&nb, nb.npages() - 1, 1860, 2480, &mut p) == 0);
    check!(p
        .as_ref()
        .is_some_and(|p| p.strokes.is_empty() && p.paper == PAPER_NARROW));
}

fn copy_and_insert_fail_without_changing_the_page_list() {
    let mut nb = NotebookInfo::default();
    let mut back = NotebookInfo::default();
    check!(store_create("Copy errors", "", PAPER_MEDIUM, &mut nb) == 0);
    let mut dir = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut dir, 700) == 0);
    let source = format!("{dir}/p1.qpg");
    let info = format!("{dir}/notebook.ini");
    let blocked = format!("{info}.tmp");
    let dest = format!("{dir}/p2.qpg");
    let original = read_file(&source);
    check!(original.is_some());
    check!(write_file_atomic(&source, b"not a page") == 0);
    check!(store_duplicate_page(&mut nb, 0, 1) != 0);
    check!(nb.npages() == 1 && nb.next_page_id == 2 && !exists(&dest));
    let data = read_file(&source);
    check!(data.as_deref() == Some(&b"not a page"[..]));
    check!(unlink(&source) && mkdir(&source));
    check!(store_duplicate_page(&mut nb, 0, 1) != 0);
    check!(nb.npages() == 1 && nb.next_page_id == 2);
    check!(std::fs::remove_dir(&source).is_ok());
    check!(original
        .as_ref()
        .is_some_and(|o| write_file_atomic(&source, o) == 0));

    // Writing the page succeeds, but saving notebook.ini fails. The new file is removed,
    // the old notebook.ini stays intact and the in-memory list is safe to save again.
    let old_info = read_file(&info);
    check!(old_info.is_some() && mkdir(&blocked));
    let next = nb.next_page_id;
    let modified = nb.modified;
    check!(store_duplicate_page(&mut nb, 0, 1) != 0);
    check!(
        nb.npages() == 1
            && nb.page_ids[0] == 1
            && nb.next_page_id == next
            && nb.modified == modified
    );
    check!(!exists(&dest));
    check!(store_add_page(&mut nb, 1, PAPER_GRID) != 0);
    check!(
        nb.npages() == 1
            && nb.page_ids[0] == 1
            && nb.next_page_id == next
            && nb.modified == modified
    );
    check!(!exists(&dest));
    let data = read_file(&info);
    check!(old_info.is_some() && data.is_some() && old_info == data);
    check!(std::fs::remove_dir(&blocked).is_ok() && store_save_info(&mut nb) == 0);
    check!(
        store_load_info(nb.id.as_str(), &mut back) == 0
            && back.npages() == 1
            && back.page_ids[0] == 1
    );
    check!(store_duplicate_page(&mut nb, 0, 1) == 0 && nb.page_ids[1] == next);
}

// rand() and srand(1) are the C library's, so the fuzz is the C test's
fn srand(seed: u32) {
    // SAFETY: srand only sets the C library's generator state.
    unsafe { libc::srand(seed) }
}
fn rand() -> usize {
    // SAFETY: rand has no preconditions.
    unsafe { libc::rand() as usize }
}

fn fuzz_deserialize() {
    let mut p = page_new(1860, 2480, PAPER_MEDIUM);
    for i in 0..5 {
        page_add(&mut p, squiggle(ToolKind(i % 3), 200.0 + i as f32 * 100.0));
    }
    let mut d = Vec::new();
    check!(page_serialize(&p, &mut d) == 0);
    let len = d.len();
    drop(p);
    let mut q = None;
    // Every truncation must fail cleanly (except the full length).
    let mut l = 0;
    while l < len {
        check!(
            page_deserialize(&d[..l], &mut q) != 0 || {
                drop(q.take());
                true
            }
        );
        l += 7;
    }
    srand(1);
    let mut m = vec![0u8; len];
    for it in 0..3000 {
        m.copy_from_slice(&d);
        for _ in 0..1 + it % 8 {
            let at = rand() % len;
            m[at] = rand() as u8;
        }
        if page_deserialize(&m, &mut q) == 0 {
            drop(q.take());
        }
    }
    for _ in 0..500 {
        for k in 0..len {
            m[k] = rand() as u8;
        }
        m[..6].copy_from_slice(b"QPG1\x01\x00");
        if page_deserialize(&m, &mut q) == 0 {
            drop(q.take());
        }
    }
}

fn custom_music_paper_roundtrip() {
    let m = MusicLayout {
        systems: 3,
        n: 5,
        part: [
            PART_TREBLE,
            PART_BASS,
            PART_TAB6,
            PART_LYRICS,
            PART_TAB4,
            PART_NONE,
        ],
    };
    let k = music_encode(&m);
    let mut r = MusicLayout::default();
    check!(paper_is_custom(k) && music_decode(k, &mut r));
    check!(r.systems == 3 && r.n == 5 && r.part[1] == PART_BASS && r.part[4] == PART_TAB4);
    check!(paper_sanitize(k.0, PAPER_BLANK) == k);
    check!(paper_sanitize(PAPER_CUSTOM_FLAG, PAPER_BLANK) == PAPER_BLANK); // no parts
    check!(paper_sanitize(PAPER_CUSTOM_FLAG | 1 << 24 | 15, PAPER_BLANK) == PAPER_BLANK); // bad part
    check!(paper_sanitize(999, PAPER_MEDIUM) == PAPER_MEDIUM);
    check!(
        music_max_systems(&m, 2480, 300.0) >= 2
            && music_max_systems(&m, 2480, 300.0) <= MUSIC_MAX_SYSTEMS
    );
    let mut p = page_new(1860, 2480, k);
    page_add(&mut p, squiggle(ToolKind(0), 300.0));
    let mut d = Vec::new();
    check!(page_serialize(&p, &mut d) == 0);
    check!(d[4] == 2); // version 2 only for custom paper
    let mut q = None;
    check!(
        page_deserialize(&d, &mut q) == 0
            && q.as_ref()
                .is_some_and(|q| q.paper == k && q.strokes.len() == 1)
    );
    let mut l = 0;
    while l < d.len() {
        let mut t = None;
        check!(
            page_deserialize(&d[..l], &mut t) != 0 || {
                drop(t.take());
                true
            }
        );
        l += 5;
    }
    drop(q);
    drop(p);
    let p = page_new(1860, 2480, PAPER_BASS5);
    check!(page_serialize(&p, &mut d) == 0 && d[4] == 1); // fixed papers stay version 1
}

fn settings_roundtrip() {
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    s.tools[TOOL_BALLPOINT.0 as usize].pressure_gamma = 2.5;
    s.tools[TOOL_PENCIL.0 as usize].tilt_size = 99.0; // out of range, must clamp on load
    s.panel_x = 123;
    s.panel_collapsed = true;
    s.pen_calib = InputCalib {
        valid: true,
        swap_xy: true,
        ax: -1860.0,
        bx: 1860.0,
        ay: 2480.0,
        by: 0.0,
    };
    check!(settings_save(&s) == 0);
    settings_load(&mut t);
    check_near!(
        t.tools[TOOL_BALLPOINT.0 as usize].pressure_gamma,
        2.5f32,
        1e-3
    );
    check_near!(t.tools[TOOL_PENCIL.0 as usize].tilt_size, 4.0f32, 1e-3);
    check!(t.panel_x == 123 && t.panel_collapsed && t.pen_calib.valid && t.pen_calib.swap_xy);
    check_near!(t.pen_calib.ax, -1860.0f32, 1e-2);
}

fn settings_upgrade_keeps_tweaks() {
    // An old settings file (no settings_version): pencil size still the old default,
    // tilt widening tuned by hand.
    let path = format!("{}/settings.ini", store_root());
    let old = "current_tool=0\npencil.size_min_mm=0.30\npencil.size_max_mm=0.85\npencil.tilt_size=3.10\nmech.grain=0.35\n";
    write_file_atomic(&path, old.as_bytes());
    let mut s = Settings::default();
    settings_load(&mut s);
    let (pencil, mech) = (TOOL_PENCIL.0 as usize, TOOL_MECH_PENCIL.0 as usize);
    check_near!(s.tools[pencil].size_min_mm, 0.12f32, 1e-3);
    check_near!(s.tools[pencil].size_max_mm, 0.32f32, 1e-3);
    check_near!(s.tools[pencil].tilt_size, 3.10f32, 1e-3); // the owner's own value stays
    check_near!(s.tools[mech].grain, 0.50f32, 1e-3);
    check!(s.version == SETTINGS_VERSION);
    // Saved again, an upgraded file never migrates twice.
    s.tools[pencil].size_max_mm = 0.85; // now a deliberate choice
    settings_save(&s);
    let mut t = Settings::default();
    settings_load(&mut t);
    check_near!(t.tools[pencil].size_max_mm, 0.85f32, 1e-3);
}

fn settings_upgrade_ballpoint_smoothing() {
    // A version-2 file: the ballpoint's smoothing still at its old default moves to the new one.
    // A pencil smoothing equal to a version-1 default stays: version 2's upgrades ran already.
    let path = format!("{}/settings.ini", store_root());
    let v2 = "settings_version=2\nballpoint.smoothing=1.0000\npencil.smoothing=0.2500\nfineliner.smoothing=1.0000\n";
    write_file_atomic(&path, v2.as_bytes());
    let mut s = Settings::default();
    settings_load(&mut s);
    let (ball, pencil, fine) = (
        TOOL_BALLPOINT.0 as usize,
        TOOL_PENCIL.0 as usize,
        TOOL_FINELINER.0 as usize,
    );
    check_near!(s.tools[ball].smoothing, 0.50f32, 1e-3);
    check_near!(s.tools[pencil].smoothing, 0.25f32, 1e-3);
    check_near!(s.tools[fine].smoothing, 1.00f32, 1e-3); // its default did not change
    check!(s.version == SETTINGS_VERSION);
    // Tuned by hand: stays.
    let tuned = "settings_version=2\nballpoint.smoothing=0.8000\n";
    write_file_atomic(&path, tuned.as_bytes());
    let mut t = Settings::default();
    settings_load(&mut t);
    check_near!(t.tools[ball].smoothing, 0.80f32, 1e-3);
    // A version-1 file gets every change.
    let v1 = "settings_version=1\nballpoint.smoothing=1.0000\npencil.smoothing=0.2500\n";
    write_file_atomic(&path, v1.as_bytes());
    let mut u = Settings::default();
    settings_load(&mut u);
    check_near!(u.tools[ball].smoothing, 0.50f32, 1e-3);
    check_near!(u.tools[pencil].smoothing, 0.00f32, 1e-3);
}

fn pen_defaults_from_device() {
    // Pen-only snapshot chosen on the Kindle, independent of the compiled defaults.
    let path = format!("{}/settings.ini", store_root());
    let snapshot = read_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/pen-defaults.ini"
    ));
    check!(snapshot.is_some());
    let Some(snapshot) = snapshot else { return };
    check!(write_file_atomic(&path, &snapshot) == 0);
    let mut chosen = Settings::default();
    let mut fresh = Settings::default();
    let mut saved = Settings::default();
    check!(settings_load(&mut chosen) == 0);
    settings_default(&mut fresh);
    check!(fresh.eraser_mode == 0); // whole strokes remain the fresh-install default
    let pi = tool_param_table();
    for t in 0..TOOL_COUNT {
        let mut reset = ToolParams::default();
        tool_params_default(ToolKind(t as i32), &mut reset); // the Pen feel reset action uses this function too
        for i in 0..pi.len() {
            let want = *tool_param_ptr(&mut chosen.tools[t], &pi[i]);
            check_near!(*tool_param_ptr(&mut fresh.tools[t], &pi[i]), want, 1e-6);
            check_near!(*tool_param_ptr(&mut reset, &pi[i]), want, 1e-6);
        }
        // A saved profile (the current version) is not migrated to these defaults.
        chosen.tools[t].size_min_mm = 0.26;
        chosen.tools[t].size_max_mm = 0.36;
        chosen.tools[t].smoothing = 0.15;
    }
    check!(settings_save(&chosen) == 0 && settings_load(&mut saved) == 0);
    for t in 0..TOOL_COUNT {
        for i in 0..pi.len() {
            check_near!(
                *tool_param_ptr(&mut saved.tools[t], &pi[i]),
                *tool_param_ptr(&mut chosen.tools[t], &pi[i]),
                1e-6
            );
        }
    }
}

// settings.ini's panel_tools: the pens pinned to the editor's panel, in pin order. A line written
// by hand or by a newer version keeps the drawing tools this version has, each once, the eraser
// left out, at most PANEL_PINS_MAX; it is written back as it was until the pins change.
fn has_pins_line(path: &str, line: &str) -> bool {
    let t = read_file(path);
    let want = format!("\n{line}\n");
    t.is_some_and(|t| find(&t, &want).is_some())
}

fn panel_tools() {
    let path = format!("{}/settings.ini", store_root());
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    check!(s.npins == 0 && !settings_pinned(&s, TOOL_PENCIL));
    check!(
        settings_toggle_pin(&mut s, TOOL_HIGHLIGHTER)
            && settings_toggle_pin(&mut s, TOOL_FINELINER)
    );
    check!(
        !settings_toggle_pin(&mut s, TOOL_ERASER)
            && !settings_toggle_pin(&mut s, ToolKind(TOOL_COUNT as i32))
    );
    check!(
        s.npins == 2 && settings_pinned(&s, TOOL_FINELINER) && !settings_pinned(&s, TOOL_ERASER)
    );
    check!(settings_save(&s) == 0 && has_pins_line(&path, "panel_tools=highlighter,fineliner"));
    check!(
        settings_load(&mut t) == 0
            && t.npins == 2
            && t.pins[0] == TOOL_HIGHLIGHTER
            && t.pins[1] == TOOL_FINELINER
    );
    // None pinned: an empty line.
    settings_default(&mut s);
    check!(settings_save(&s) == 0 && has_pins_line(&path, "panel_tools="));
    check!(settings_load(&mut t) == 0 && t.npins == 0);
    // Unknown names, the eraser and repeats are left out; the first three left are the pins.
    let mixed = "panel_tools=eraser,nope,highlighter,highlighter,pencil,mech,ballpoint,marker\n";
    check!(write_file_atomic(&path, mixed.as_bytes()) == 0);
    check!(settings_load(&mut t) == 0 && t.npins == 3);
    check!(
        t.pins[0] == TOOL_HIGHLIGHTER
            && t.pins[1] == TOOL_PENCIL
            && t.pins[2] == TOOL_MECH_PENCIL

    );
    for i in 0..t.npins as usize {
        for k in 0..i {
            check!(t.pins[i] != t.pins[k] && t.pins[i] != TOOL_ERASER);
        }
    }
    let mut text = String::new();
    settings_pins_text(&t, &mut text, 128);
    check!(text == "highlighter,pencil,mech");
    // Full: a fourth is refused, the pins stay as they were.
    check!(
        !settings_toggle_pin(&mut t, TOOL_MARKER)
            && t.npins == 3
            && !settings_pinned(&t, TOOL_MARKER)
    );
    // Saved unchanged, the line goes back as it was (a newer version's pen is still in it).
    check!(
        settings_save(&t) == 0
            && has_pins_line(
                &path,
                "panel_tools=eraser,nope,highlighter,highlighter,pencil,mech,ballpoint,marker"
            )
    );
    // Changed, the pins are written as they are: unpinning one keeps the others' order.
    check!(settings_toggle_pin(&mut t, TOOL_PENCIL) && t.npins == 2);
    check!(
        settings_save(&t) == 0 && has_pins_line(&path, "panel_tools=highlighter,mech")
    );
    check!(settings_toggle_pin(&mut t, TOOL_CALLIGRAPHY) && t.pins[2] == TOOL_CALLIGRAPHY);
    check!(
        settings_save(&t) == 0
            && has_pins_line(&path, "panel_tools=highlighter,mech,calligraphy")
    );
    check!(settings_load(&mut s) == 0 && s.npins == 3 && s.pins[2] == TOOL_CALLIGRAPHY);
    // Written by hand with spaces, and with nothing between commas.
    let spaced = "panel_tools= fineliner ,, marker,\n";
    check!(write_file_atomic(&path, spaced.as_bytes()) == 0);
    check!(
        settings_load(&mut t) == 0
            && t.npins == 2
            && t.pins[0] == TOOL_FINELINER
            && t.pins[1] == TOOL_MARKER
    );
    unlink(&path);
}

// settings.ini's before_eraser: the pen in use when the eraser was picked, -1 for none. A pen this
// version lacks, the eraser or no number is none, and the line goes back as it was until it changes.
fn before_eraser() {
    let path = format!("{}/settings.ini", store_root());
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    check!(s.before_eraser == -1);
    check!(settings_save(&s) == 0 && has_pins_line(&path, "before_eraser=-1"));
    check!(settings_load(&mut t) == 0 && t.before_eraser == -1);
    s.before_eraser = TOOL_MARKER.0;
    check!(settings_save(&s) == 0 && has_pins_line(&path, "before_eraser=7"));
    check!(settings_load(&mut t) == 0 && t.before_eraser == TOOL_MARKER.0);
    let odd = ["12", "3", "pencil", "4x", ""];
    for i in 0..odd.len() {
        let f = format!("before_eraser={}\n", odd[i]);
        check!(write_file_atomic(&path, f.as_bytes()) == 0);
        check!(settings_load(&mut t) == 0 && t.before_eraser == -1);
        let line = format!("before_eraser={}", odd[i]);
        check!(settings_save(&t) == 0 && has_pins_line(&path, &line)); // written back as it was
    }
    // Changed, it is written as it is.
    t.before_eraser = TOOL_HIGHLIGHTER.0;
    check!(settings_save(&t) == 0 && has_pins_line(&path, "before_eraser=4"));
    check!(settings_load(&mut s) == 0 && s.before_eraser == TOOL_HIGHLIGHTER.0);
    unlink(&path);
}

// The clock and battery corner's keys (status.rs; Rust only): off and small without them, written
// last of the known keys, clamped, and a value past the range or a third state written back as it
// was until the setting changes.
fn shape_preferences_and_retired_bar_migration() {
    let path=format!("{}/settings.ini",store_root());
    check!(write_file_atomic(&path,b"status_mode=0\nstatus_on=0\nstatus_size=2\nstatus_corner=3\nshape_quads=2\nshape_round=2\nshape_links=0\nshape_hold_ms=900\n")==0);
    let mut s=Settings::default();check!(settings_load(&mut s)==0);
    check!(s.status_mode==1 && s.status_on && s.status_size==0 && s.status_corner==0);
    check!(s.shape_quads==2 && s.shape_round==2 && !s.shape_links && s.shape_hold_ms==900);
    s.status_on=false;check!(settings_save(&s)==0);
    let mut t=Settings::default();check!(settings_load(&mut t)==0);
    check!(!t.status_on && t.status_mode==1 && t.shape_quads==2 && t.shape_round==2 && !t.shape_links);
}

// Corner roundness (2.0.0-rc.12): sharp without the key, clamped to 0..100, a value past the range
// written back as it was until the setting changes, the last known key.
fn shape_corner_key() {
    let path = format!("{}/settings.ini", store_root());
    let mut s = Settings::default();
    check!(write_file_atomic(&path, b"invert=1\n") == 0);
    check!(settings_load(&mut s) == 0 && s.shape_corner == 0);
    check!(write_file_atomic(&path, b"shape_corner=35\n") == 0);
    check!(settings_load(&mut s) == 0 && s.shape_corner == 35);
    check!(write_file_atomic(&path, b"shape_corner=150\n") == 0);
    check!(settings_load(&mut s) == 0 && s.shape_corner == 100);
    check!(settings_save(&s) == 0 && has_pins_line(&path, "shape_corner=150"));
    s.shape_corner = 40;
    check!(settings_save(&s) == 0 && has_pins_line(&path, "shape_corner=40"));
    let text = read_file(&path).unwrap_or_default();
    let at = |k: &str| text.windows(k.len()).position(|w| w == k.as_bytes());
    check!(at("whatsnew_hide=") < at("shape_corner="));
    check!(write_file_atomic(&path, b"shape_corner=-3\n") == 0);
    check!(settings_load(&mut s) == 0 && s.shape_corner == 0);
}

fn status_corner_keys() {
    let path = format!("{}/settings.ini", store_root());
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    // (medium by default since 2.0.0-rc.12)
    check!(s.status_on && s.status_size == 1 && s.status_corner == 0 && s.status_mode == 1);
    check!(
        settings_save(&s) == 0
            && has_pins_line(&path, "status_on=1")
            && has_pins_line(&path, "status_size=1")
    );
    let text = read_file(&path).unwrap_or_default();
    let at = |k: &str| text.windows(k.len()).position(|w| w == k.as_bytes());
    check!(at("language=") < at("status_on=") && at("status_on=") < at("status_size="));
    s.status_on = true;
    s.status_size = 2;
    check!(
        settings_save(&s) == 0
            && has_pins_line(&path, "status_on=1")
            && has_pins_line(&path, "status_size=2")
    );
    check!(settings_load(&mut t) == 0 && t.status_on && t.status_size == 2);
    // a file with neither key: the defaults
    check!(write_file_atomic(&path, b"invert=1\n") == 0);
    check!(settings_load(&mut t) == 0 && t.status_on && t.status_size == 1);
    // a size past the range is the nearest, a third state is on; both go back as they were
    check!(write_file_atomic(&path, b"status_on=2\nstatus_size=7\n") == 0);
    check!(settings_load(&mut t) == 0 && t.status_on && t.status_size == 2);
    check!(
        settings_save(&t) == 0
            && has_pins_line(&path, "status_on=2")
            && has_pins_line(&path, "status_size=7")
    );
    // changed, they are written as they are
    t.status_size = 1;
    t.status_on = false;
    check!(
        settings_save(&t) == 0
            && has_pins_line(&path, "status_on=0")
            && has_pins_line(&path, "status_size=1")
    );
    check!(settings_load(&mut s) == 0 && !s.status_on && s.status_size == 1);
    unlink(&path);
}

// The gestures' keys (Index phase 1; Rust only): on, on, none and off without them, written after the
// clock corner's, clamped, a third state or a value past the range written back as it was until the
// setting changes. `upgraded` is memory only: an existing version 3 file without a gestures key.
fn gesture_keys() {
    let path = format!("{}/settings.ini", store_root());
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    check!(s.gestures && s.landscape_fit && s.hints == 0 && !s.link_areas && !s.upgraded);
    check!(settings_save(&s) == 0);
    for l in ["gestures=1", "landscape_fit=1", "hints=0", "link_areas=0"] {
        check!(has_pins_line(&path, l));
    }
    let text = read_file(&path).unwrap_or_default();
    let at = |k: &str| text.windows(k.len()).position(|w| w == k.as_bytes());
    check!(at("language=") < at("status_size=") && at("status_size=") < at("gestures="));
    check!(
        at("gestures=") < at("landscape_fit=")
            && at("landscape_fit=") < at("hints=")
            && at("hints=") < at("link_areas=")
    );
    check!(!text.windows(9).any(|w| w == b"upgraded="));
    s.gestures = false;
    s.landscape_fit = false;
    s.hints = 0x1c5;
    s.link_areas = true;
    check!(
        settings_save(&s) == 0
            && has_pins_line(&path, "gestures=0")
            && has_pins_line(&path, "hints=453")
    );
    check!(
        settings_load(&mut t) == 0
            && !t.gestures
            && !t.landscape_fit
            && t.hints == 453
            && t.link_areas
    );
    // a file with none of them: the defaults, and not an upgrade (it has no settings_version either)
    check!(write_file_atomic(&path, b"invert=1\n") == 0);
    check!(
        settings_load(&mut t) == 0
            && t.gestures
            && t.landscape_fit
            && t.hints == 0
            && !t.link_areas
            && !t.upgraded
    );
    // hints is clamped to 0..i32::MAX; a third state of a switch is on; both go back as they were
    check!(
        write_file_atomic(
            &path,
            b"gestures=2\nlandscape_fit=0\nhints=-5\nlink_areas=3\n"
        ) == 0
    );
    check!(
        settings_load(&mut t) == 0
            && t.gestures
            && !t.landscape_fit
            && t.hints == 0
            && t.link_areas
    );
    check!(
        settings_save(&t) == 0
            && has_pins_line(&path, "gestures=2")
            && has_pins_line(&path, "hints=-5")
    );
    check!(has_pins_line(&path, "link_areas=3") && has_pins_line(&path, "landscape_fit=0"));
    // changed, they are written as they are
    t.hints = 64;
    t.gestures = false;
    check!(
        settings_save(&t) == 0
            && has_pins_line(&path, "hints=64")
            && has_pins_line(&path, "gestures=0")
    );
    // The clock corner's place (status_corner): the top left by default, written after link_areas, clamped to
    // 0..3 (a number past it is the nearest and goes back as it was), a third state not a thing for it.
    check!(s.status_corner == 0);
    check!(settings_save(&s) == 0 && has_pins_line(&path, "status_corner=0"));
    let text = read_file(&path).unwrap_or_default();
    let at = |k: &str| text.windows(k.len()).position(|w| w == k.as_bytes());
    check!(at("link_areas=") < at("status_corner="));
    s.status_corner = 3;
    check!(settings_save(&s) == 0 && has_pins_line(&path, "status_corner=3"));
    check!(settings_load(&mut t) == 0 && t.status_corner == 3);
    check!(write_file_atomic(&path, b"status_corner=9\n") == 0);
    check!(settings_load(&mut t) == 0 && t.status_corner == 3);
    check!(settings_save(&t) == 0 && has_pins_line(&path, "status_corner=9"));
    check!(write_file_atomic(&path, b"status_corner=-2\n") == 0);
    check!(settings_load(&mut t) == 0 && t.status_corner == 0);
    check!(write_file_atomic(&path, b"invert=1\n") == 0);
    check!(settings_load(&mut t) == 0 && t.status_corner == 0);
    // upgraded: version 3 exactly, a file that exists, no gestures key
    check!(write_file_atomic(&path, b"settings_version=3\ninvert=1\n") == 0);
    check!(settings_load(&mut t) == 0 && t.upgraded);
    check!(write_file_atomic(&path, b"settings_version=3\ngestures=1\n") == 0);
    check!(settings_load(&mut t) == 0 && !t.upgraded);
    check!(write_file_atomic(&path, b"settings_version=2\ninvert=1\n") == 0);
    check!(settings_load(&mut t) == 0 && !t.upgraded);
    check!(write_file_atomic(&path, b"invert=1\n") == 0);
    check!(settings_load(&mut t) == 0 && !t.upgraded);
    unlink(&path);
    check!(settings_load(&mut t) != 0 && !t.upgraded);
    // a file this build saved is not an upgrade the next time
    settings_default(&mut s);
    check!(settings_save(&s) == 0 && settings_load(&mut t) == 0 && !t.upgraded);
    unlink(&path);
}

// Rotation lock's settings.ini value is 0 (follow) or the locked orientation plus one.
fn rotation_lock_key() {
    let path = format!("{}/settings.ini", store_root());
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    check!(
        s.rotation_lock == 0 && settings_save(&s) == 0 && has_pins_line(&path, "rotation_lock=0")
    );
    let text = read_file(&path).unwrap_or_default();
    let at = |k: &str| text.windows(k.len()).position(|w| w == k.as_bytes());
    check!(
        at("status_corner=") < at("rotation_lock=")
            && at("rotation_lock=") < at("gesture_new_page=")
    );
    s.rotation_lock = 4;
    check!(settings_save(&s) == 0 && settings_load(&mut t) == 0 && t.rotation_lock == 4);
    check!(write_file_atomic(&path, b"rotation_lock=7\n") == 0);
    check!(settings_load(&mut t) == 0 && t.rotation_lock == 4);
    check!(settings_save(&t) == 0 && has_pins_line(&path, "rotation_lock=7"));
    check!(write_file_atomic(&path, b"rotation_lock=-2\n") == 0);
    check!(settings_load(&mut t) == 0 && t.rotation_lock == 0);
    check!(write_file_atomic(&path, b"invert=1\n") == 0);
    check!(settings_load(&mut t) == 0 && t.rotation_lock == 0);
    unlink(&path);
}

// A damaged page is renamed aside, never overwritten by the blank shown in its place; an empty
// notebook.ini gets its page list back from the page files.
fn damage_is_kept() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Fragile", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0);
    check!(store_add_page(&mut nb, 2, PAPER_MEDIUM) == 0);
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    let path = format!("{dir}/p2.qpg");
    check!(std::fs::write(&path, b"QPG1 this is not a page").is_ok());
    let mut p = None;
    check!(
        store_load_page(&nb, 1, 1860, 2480, &mut p) == 0
            && p.as_ref().is_some_and(|p| p.strokes.is_empty())
    );
    drop(p);
    check!(store_take_corrupt_count() == 1);
    check!(store_take_corrupt_count() == 0);
    check!(std::fs::metadata(&path).is_err()); // moved aside...
    let mut aside = 0;
    if let Ok(d) = std::fs::read_dir(&dir) {
        for e in d.flatten() {
            if e.file_name()
                .to_string_lossy()
                .starts_with("p2.qpg.damaged-")
            {
                aside += 1;
            }
        }
    }
    check!(aside == 1); // ...not deleted

    let path = format!("{dir}/notebook.ini");
    let _ = std::fs::write(&path, b""); // truncated to nothing
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0);
    check!(back.npages() == 2 && back.page_ids[0] == 1 && back.page_ids[1] == 3);
    // p2 is set aside
}

static PUMPS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
fn pumped() {
    PUMPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

// The editor's pumped reader (a peek reads a page while the pens are read): the same page as store_load_page
// reads, stroke for stroke, calling work_pump() at least once per 256 strokes as it goes, where the plain
// reader never does; a damaged file is set aside by either, and counted.
fn a_pumped_load_is_the_same_page_and_pumps() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Peek", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0);
    let mut p = None;
    check!(store_load_page(&nb, 0, 1860, 2480, &mut p) == 0);
    let mut pg = p.unwrap();
    for i in 0..700 {
        page_add(
            &mut pg,
            squiggle(
                if i % 2 == 0 {
                    TOOL_PENCIL
                } else {
                    TOOL_BALLPOINT
                },
                100.0 + (i % 40) as f32 * 50.0,
            ),
        );
    }
    check!(store_save_page(&nb, 0, &pg) == 0);
    let (mut plain, mut pump) = (None, None);
    PUMPS.store(0, std::sync::atomic::Ordering::Relaxed);
    work_pump_set(Some(pumped));
    check!(store_load_page(&nb, 0, 1860, 2480, &mut plain) == 0);
    let after_plain = PUMPS.load(std::sync::atomic::Ordering::Relaxed);
    check!(store_load_page_pumped(&nb, 0, 1860, 2480, &mut pump) == 0);
    let after_pumped = PUMPS.load(std::sync::atomic::Ordering::Relaxed);
    work_pump_set(None);
    check!(after_plain == 0 && after_pumped - after_plain >= 700 / 256);
    let (a, b) = (plain.unwrap(), pump.unwrap());
    check!(
        a.strokes.len() == 700
            && b.strokes.len() == 700
            && a.paper == b.paper
            && a.w == b.w
            && a.h == b.h
            && a.readonly == b.readonly
    );
    for i in 0..a.strokes.len() {
        let (x, y) = (&a.strokes[i], &b.strokes[i]);
        check!(
            x.tool == y.tool && x.seed == y.seed && x.pts.len() == y.pts.len() && x.bbox == y.bbox
        );
        check!(
            x.pts[0].x == y.pts[0].x
                && x.pts[49].y == y.pts[49].y
                && x.pts[25].pressure == y.pts[25].pressure
        );
    }
    // a damaged file: the pumped reader sets it aside as an open does, and a blank page takes its place
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    check!(std::fs::write(format!("{dir}/p2.qpg"), b"QPG1 this is not a page").is_ok());
    store_take_corrupt_count();
    let mut q = None;
    check!(
        store_load_page_pumped(&nb, 1, 1860, 2480, &mut q) == 0
            && q.as_ref().is_some_and(|q| q.strokes.is_empty())
    );
    check!(store_take_corrupt_count() == 1 && std::fs::metadata(format!("{dir}/p2.qpg")).is_err());
    // out of range: nothing, as store_load_page
    check!(store_load_page_pumped(&nb, 9, 1860, 2480, &mut q) == -1 && q.is_none());
}

// Folders sort by their letters, accents or not: an accented first letter sorts with its base
// letter, not after Z (the keyboard types accents).
fn folders_sort_with_accents() {
    let names = [
        "Zoología",
        "Álgebra",
        "Arte",
        "Études",
        "school",
        "Straße",
        "Strasse",
        "Łódź",
        "Lapiz",
        "e\u{301}poca",
    ];
    for n in names.iter() {
        check!(store_add_folder(n) == 0);
    }
    let mut fs: Vec<String> = Vec::new();
    check!(store_list_folders(&mut fs) == 0);
    // (ß sorts as ss; equal letters, the bytes decide: "Strasse" before "Straße"; a decomposed
    // e-acute sorts as e.)
    let want = [
        "Álgebra",
        "Arte",
        "e\u{301}poca",
        "Études",
        "Lapiz",
        "Łódź",
        "school",
        "Strasse",
        "Straße",
        "Zoología",
    ];
    let mut at = 0;
    for i in 0..fs.len() {
        if at >= want.len() {
            break;
        }
        if fs[i] == want[at] {
            at += 1;
        } else {
            for k in 0..want.len() {
                if fs[i] == want[k] {
                    eprintln!("  folder {}: {} (wanted {})", i, fs[i], want[at]);
                }
            }
        }
    }
    check!(at == want.len()); // all of them, in this order
    for n in names.iter() {
        store_remove_folder(n);
    }
}

fn folder_named_like_a_comment() {
    check!(store_add_folder("#1") == 0);
    let mut fs: Vec<String> = Vec::new();
    check!(store_list_folders(&mut fs) == 0);
    let mut found = false;
    for f in &fs {
        found |= f == "#1";
    }
    check!(found);
}

fn safe_names() {
    let mut out = String::new();
    export_safe_name("a/b:c?", &mut out, 64);
    check!(out == "a_b_c_");
    export_safe_name("  ..  ", &mut out, 64);
    check!(out == "Untitled");
}

fn empty_mirror_folder_cleanup() {
    use std::os::unix::fs::symlink;
    let mirror = format!("{}/prune-mirror", root());
    check!(mkdir_p(&mirror) == 0);
    let mut path = format!("{mirror}/Gone");
    check!(mkdir_p(&path) == 0 && store_add_folder("Gone") == 0);
    check!(export_mirror_prune_folder("Gone", &mirror) == 0 && exists(&path));
    check!(store_remove_folder("Gone") == 0);
    check!(export_mirror_prune_folder("Gone", &mirror) == 1 && !exists(&path));
    check!(export_mirror_prune_folder("", &mirror) == 0 && exists(&mirror));
    // Different logical names can share the same sanitized directory.
    path = format!("{mirror}/Shared_Name");
    check!(mkdir_p(&path) == 0 && store_add_folder("Shared:Name") == 0);
    check!(export_mirror_prune_folder("Shared/Name", &mirror) == 0 && exists(&path));
    check!(store_remove_folder("Shared:Name") == 0);
    check!(export_mirror_prune_folder("Shared/Name", &mirror) == 1);
    // Files and KOReader's nested sidecars are never removed recursively.
    path = format!("{mirror}/Kept/book.sdr");
    check!(mkdir_p(&path) == 0);
    check!(export_mirror_prune_folder("Kept", &mirror) == 0 && exists(&path));
    path = format!("{mirror}/Kept/notes.txt");
    check!(write_file_atomic(&path, b"keep") == 0);
    check!(export_mirror_prune_folder("Kept", &mirror) == 0 && exists(&path));
    // No following either a candidate directory symlink or a symlink mirror root.
    let outside = format!("{}/outside", root());
    check!(mkdir_p(&outside) == 0);
    let mut link = format!("{mirror}/Link");
    let absolute_outside = std::fs::canonicalize(&outside);
    check!(absolute_outside.is_ok());
    check!(absolute_outside
        .as_ref()
        .is_ok_and(|a| symlink(a, &link).is_ok()));
    check!(export_mirror_prune_folder("Link", &mirror) == 0 && exists(&outside));
    check!(std::fs::symlink_metadata(&link).is_ok_and(|m| m.file_type().is_symlink()));
    path = format!("{mirror}/Empty");
    check!(mkdir_p(&path) == 0);
    link = format!("{}/mirror-link", root());
    let absolute_mirror = std::fs::canonicalize(&mirror);
    check!(absolute_mirror.is_ok());
    check!(absolute_mirror
        .as_ref()
        .is_ok_and(|a| symlink(a, &link).is_ok()));
    check!(export_mirror_prune_folder("Empty", &link) == -1 && exists(&path));
    check!(export_mirror_prune_folder("../outside", &mirror) == 0 && exists(&outside));
    // A remaining logical folder may refer to the candidate through an alias.
    path = format!("{mirror}/AliasTarget");
    check!(mkdir_p(&path) == 0);
    let absolute_target = std::fs::canonicalize(&path);
    check!(absolute_target.is_ok());
    link = format!("{mirror}/Alias");
    check!(absolute_target
        .as_ref()
        .is_ok_and(|a| symlink(a, &link).is_ok()));
    check!(store_add_folder("Alias") == 0);
    check!(export_mirror_prune_folder("AliasTarget", &mirror) == 0 && exists(&path));
    check!(store_remove_folder("Alias") == 0);
    check!(export_mirror_prune_folder("AliasTarget", &mirror) == 1);
}

#[test]
fn test_store() {
    let r = format!("{}/store-{}", tl_tmp(), std::process::id());
    let _ = ROOT.set(r.clone());
    let _ = std::fs::remove_dir_all(&r);
    check!(store_init(&r) == 0);
    run!(notebooks);
    run!(buffer_failure_stays_failed);
    run!(new_page_after_a_document_page);
    run!(copy_pages_keeps_bytes_and_order);
    run!(copy_and_insert_fail_without_changing_the_page_list);
    run!(fuzz_deserialize);
    run!(custom_music_paper_roundtrip);
    run!(settings_roundtrip);
    run!(settings_upgrade_keeps_tweaks);
    run!(settings_upgrade_ballpoint_smoothing);
    run!(pen_defaults_from_device);
    run!(panel_tools);
    run!(before_eraser);
    run!(status_corner_keys);
    run!(shape_preferences_and_retired_bar_migration);
    run!(shape_corner_key);
    run!(gesture_keys);
    run!(rotation_lock_key);
    run!(safe_names);
    run!(empty_mirror_folder_cleanup);
    run!(damage_is_kept);
    run!(a_pumped_load_is_the_same_page_and_pumps);
    run!(folder_named_like_a_comment);
    run!(folders_sort_with_accents);
    let _ = std::fs::remove_dir_all(&r);
    done();
}
