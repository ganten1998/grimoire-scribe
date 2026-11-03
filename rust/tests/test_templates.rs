// test_templates.rs: image papers (pictures as page backgrounds), templates and imports
// (src/bgimage.h, src/templates.h, docs/FORMAT.md, Image papers). The port of tests/test_templates.c:
// the same tests, in the C's order, with the same checks.
#![allow(clippy::needless_range_loop)] // (the C's index loops, kept)
mod beta_format;
mod testlib;
use beta_format::*;
use grimoire::prelude::*;
use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
fn root() -> String {
    ROOT.get().cloned().unwrap_or_default()
}
const DPI: f32 = 300.0;
// the page size the tests' notebooks get (store_set_device in main)
const PW: i32 = 600;
const PH: i32 = 800;

fn fixture(name: &str) -> String {
    format!("{}/../tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

// ---- the system, as the C test calls it ----

fn system(cmd: &str) -> i32 {
    let c = CString::new(cmd).unwrap();
    // SAFETY: system reads the NUL-terminated command we own.
    unsafe { libc::system(c.as_ptr()) }
}

fn geteuid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

fn chmod(path: &str, mode: u32) -> i32 {
    if std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).is_ok() {
        0
    } else {
        -1
    }
}

fn mkdir(path: &str) -> i32 {
    if std::fs::create_dir(path).is_ok() {
        0
    } else {
        -1
    }
}

fn unlink(path: &str) {
    let _ = std::fs::remove_file(path);
}

fn usleep(us: u64) {
    std::thread::sleep(std::time::Duration::from_micros(us));
}

// ---- making pictures ----

static G_RNG: AtomicU32 = AtomicU32::new(12345);
fn rnd8() -> u8 {
    let g = G_RNG.load(Ordering::Relaxed).wrapping_mul(1103515245).wrapping_add(12345);
    G_RNG.store(g, Ordering::Relaxed);
    (g >> 16) as u8
}

fn picture(w: i32, h: i32, ch: i32) -> BgPicture {
    BgPicture { px: vec![0; w as usize * h as usize * ch as usize], w, h, ch }
}

// Noise: any filter or offset mistake shows.
fn noise(w: i32, h: i32, ch: i32) -> BgPicture {
    let mut p = picture(w, h, ch);
    for i in 0..w as usize * h as usize * ch as usize {
        p.px[i] = rnd8();
    }
    p
}

fn solid(w: i32, h: i32, ch: i32, v: u8) -> BgPicture {
    let mut p = picture(w, h, ch);
    p.px.fill(v);
    p
}

fn put_file(path: &str, d: &[u8]) {
    let dir = match path.rfind('/') {
        Some(s) => &path[..s],
        None => path,
    };
    mkdir_p(dir);
    check!(write_file_atomic(path, d) == 0);
}

fn put_png(path: &str, p: &BgPicture) {
    let dir = match path.rfind('/') {
        Some(s) => &path[..s],
        None => path,
    };
    mkdir_p(dir);
    check!(bgimage_write_png(path, p) == 0);
}

// Writes picture p as notebook nb's bg/<id>.png, as a template's or an import's would be.
fn put_picture(nb: &NotebookInfo, id: u32, p: &BgPicture) {
    let (mut dir, mut file) = (String::new(), String::new());
    check!(bgimage_dir(nb.id.as_str(), &mut dir, 700) == 0);
    bgimage_file(&dir, id, &mut file, 800);
    put_png(&file, p);
}

fn slurp(path: &str) -> Option<Vec<u8>> {
    read_file(path)
}

// A file's text, for strstr and strcmp.
fn slurp_text(path: &str) -> Option<String> {
    slurp(path).map(|d| String::from_utf8_lossy(&d).into_owned())
}

// A PNG of `bytes` bytes or more: picture p, with a chunk of its own (an ancillary one, which
// decoders skip) of zeros after its header, as a big ICC profile or text would be.
fn padded_png(path: &str, p: &BgPicture, bytes: usize) {
    let tmp = format!("{path}.plain.png");
    put_png(&tmp, p);
    let png = slurp(&tmp);
    unlink(&tmp);
    if let Some(png) = png {
        let len = png.len();
        let mut out = vec![0u8; len + bytes + 12];
        out[..33].copy_from_slice(&png[..33]); // the signature and IHDR
        out[33..37].copy_from_slice(&(bytes as u32).to_be_bytes());
        out[37..41].copy_from_slice(b"prvT");
        out[33 + 8 + bytes + 4..].copy_from_slice(&png[33..]); // after its CRC: the rest
        put_file(path, &out);
    }
}

fn exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

// What the code under test logs (grimoire.log is its stderr), from log_capture to log_captured
// (the text).
static G_LOG_SAVED: AtomicI32 = AtomicI32::new(-1);
static G_LOG_PATH: Mutex<String> = Mutex::new(String::new());

fn log_capture() {
    use std::io::Write;
    let path = format!("{}/captured.log", root());
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns (stderr's, and the log file's).
    unsafe {
        G_LOG_SAVED.store(libc::dup(2), Ordering::Relaxed);
        let c = CString::new(path.as_str()).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
    *G_LOG_PATH.lock().unwrap() = path;
}

fn log_captured() -> Option<String> {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    let saved = G_LOG_SAVED.swap(-1, Ordering::Relaxed);
    if saved >= 0 {
        // SAFETY: dup2 and close on the descriptor log_capture saved of stderr.
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
    let path = G_LOG_PATH.lock().unwrap().clone();
    let t = slurp_text(&path);
    if let Some(t) = &t {
        eprint!("{t}"); // (still in the test's own log)
    }
    t
}

// The entries of dir whose names start with prefix ("." and ".." are entries too, as readdir has them).
fn count_prefix(dir: &str, prefix: &str) -> i32 {
    let mut n = 0;
    for special in [".", ".."] {
        if special.starts_with(prefix) {
            n += 1;
        }
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with(prefix) {
                n += 1;
            }
        }
    }
    n
}

// Waits for the background jobs to finish (sanitized children on a busy machine: by the clock).
fn drain_jobs(last_kind: Option<&mut i32>) -> i32 {
    let mut last_kind = last_kind;
    let mut result = 0;
    let until = now_ms().wrapping_add(60000);
    while bg_busy() && (now_ms().wrapping_sub(until) as i32) < 0 {
        let mut kind = -1;
        let r = bg_poll_kind(Some(&mut kind));
        if kind >= 0 {
            if let Some(k) = last_kind.as_mut() {
                **k = kind;
            }
        }
        if r != 0 {
            result = r;
        }
        usleep(5000);
    }
    let mut kind = -1;
    let r = bg_poll_kind(Some(&mut kind));
    if kind >= 0 {
        if let Some(k) = last_kind.as_mut() {
            **k = kind;
        }
    }
    if r != 0 {
        r
    } else {
        result
    }
}

fn line_stroke(t: ToolKind, y: f32, seed: u32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    let mut s = stroke_new(t, &p, seed);
    for i in 0..30 {
        let q = PenSample {
            x: fmla(i as f32, 17.0, 40.0),
            y: fmla((i % 4) as f32, 6.0, y),
            pressure: 0.3 + i as f32 / 60.0,
            tilt_x: 4.0,
            tilt_y: -3.0,
            t_ms: i as u32 * 7,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// ---- the paper values ----

fn image_kinds() {
    let t = paper_image(PAPER_ROLE_TEMPLATE, 0x0A1B2C);
    let d = paper_image(PAPER_ROLE_DOCUMENT, 0x0A1B2C);
    check!(t.0 == 0x500A1B2C && d.0 == 0x510A1B2C);
    check!(paper_is_image(t) && paper_is_image(d) && !paper_is_document(t) && paper_is_document(d));
    check!(paper_image_id(t) == 0x0A1B2C && paper_image_id(d) == 0x0A1B2C);
    check!(paper_is_known(t) && paper_is_known(d) && paper_is_known(PaperKind(0x50000000)));
    // Roles 2 and 3, and the colour bits (26, 27): a later version's paper.
    let newer: [u32; 6] = [0x52000001, 0x53000001, 0x54000001, 0x58000001, 0x5C000001, 0x5F000001];
    for n in newer {
        check!(!paper_is_image(PaperKind(n)) && !paper_is_known(PaperKind(n)));
        check!(paper_name(PaperKind(n)) == "Newer paper");
    }
    check!(paper_name(t) == "Template" && paper_name(d) == "Imported page");
    // A new notebook or a setting never takes one: it needs its notebook's picture.
    check!(paper_sanitize(t.0, PAPER_MEDIUM) == PAPER_MEDIUM);
    check!(paper_sanitize(d.0, PAPER_BLANK) == PAPER_BLANK);
    let mut nb = NotebookInfo::default();
    check!(store_create("Fresh", "", t, &mut nb) == 0 && nb.paper == PAPER_MEDIUM);
    // Drawn alone (no notebook), an image paper is blank, like a paper this version does not know.
    let (mut a, mut b) = (Canvas::default(), Canvas::default());
    canvas_init(&mut a, 60, 80);
    canvas_init(&mut b, 60, 80);
    canvas_fill(&mut a, rect(0, 0, 60, 80), 0);
    paper_take_image_blank_count();
    paper_render(&mut a, t, rect(0, 0, 60, 80), 30.0);
    check!(a.px[..60 * 80] == b.px[..60 * 80]);
    check!(paper_take_image_blank_count() == 1 && paper_take_image_blank_count() == 0); // (a caller to fix)
    canvas_free(&mut a);
    canvas_free(&mut b);
    // Page version 2, the u16 Blank (1.0.0-beta's reading of it), the value kept.
    let p = page_new(PW, PH, d);
    let mut data = Vec::new();
    check!(page_serialize(&p, &mut data) == 0);
    check!(data[4] == 2 && data[5] == 0 && data[6] == PAPER_BLANK.0 as u8 && data[7] == 0);
    let mut q: Option<Box<Page>> = None;
    check!(page_deserialize(&data, &mut q) == 0 && q.as_ref().is_some_and(|q| q.paper == d && !q.readonly));
    if let Some(q) = q {
        page_free(q);
    }
    page_free(p);
}

// ---- drawing ----

// Renders page p whole, in four tiles and in random rects: all three agree, pixel for pixel.
fn check_pure(p: &Page, what: &str) {
    let (w, h) = (p.w, p.h);
    let (mut full, mut tiles, mut part) = (Canvas::default(), Canvas::default(), Canvas::default());
    canvas_init(&mut full, w, h);
    canvas_init(&mut tiles, w, h);
    canvas_init(&mut part, w, h);
    page_render(&mut full, p, rect(0, 0, w, h), DPI);
    canvas_fill(&mut tiles, rect(0, 0, w, h), 7);
    let (sx, sy) = (w / 3 + 1, h / 2 - 3);
    let q = [rect(0, 0, sx, sy), rect(sx, 0, w - sx, sy), rect(0, sy, sx, h - sy), rect(sx, sy, w - sx, h - sy)];
    for rc in q {
        page_render(&mut tiles, p, rc, DPI);
    }
    let mut same = full.px[..w as usize * h as usize] == tiles.px[..w as usize * h as usize];
    let mut i = 0;
    while i < 40 && same {
        let x = (rnd8() as u32 * w as u32 / 256) as i32;
        let y = (rnd8() as u32 * h as u32 / 256) as i32;
        let rw = 1 + (rnd8() as u32 % 200) as i32;
        let rh = 1 + (rnd8() as u32 % 200) as i32;
        let rc = rect_intersect(rect(x, y, rw, rh), rect(0, 0, w, h));
        canvas_fill(&mut part, rect(0, 0, w, h), 3);
        page_render(&mut part, p, rc, DPI);
        let mut yy = rc.y;
        while yy < rc.y + rc.h && same {
            let a = (yy * part.stride + rc.x) as usize;
            let b = (yy * full.stride + rc.x) as usize;
            same = part.px[a..a + rc.w as usize] == full.px[b..b + rc.w as usize];
            yy += 1;
        }
        i += 1;
    }
    if !same {
        eprintln!("  {what}: a region drawn alone differs from the whole page");
    }
    check!(same);
    canvas_free(&mut full);
    canvas_free(&mut tiles);
    canvas_free(&mut part);
}

fn image_paper_is_pure() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Pure", "", PAPER_BLANK, &mut nb) == 0);
    let mut src = noise(500, 700, 3);
    let mut fit = BgPicture::default();
    check!(bgimage_normalize(&src, PW, PH, &mut fit) == 0 && fit.w == PW && fit.h == PH && fit.ch == 3);
    put_picture(&nb, 0x123456, &fit);
    let mut p = page_new(PW, PH, paper_image(PAPER_ROLE_TEMPLATE, 0x123456));
    p.nb.set(nb.id.as_str());
    // The paper alone is the picture's luma, exactly.
    let (mut paper, mut luma) = (Canvas::default(), Canvas::default());
    canvas_init(&mut paper, PW, PH);
    page_paper_render(&mut paper, &p, rect(0, 0, PW, PH), DPI);
    check!(bgimage_luma(&fit, &mut luma) == 0 && paper.px[..(PW * PH) as usize] == luma.px[..(PW * PH) as usize]);
    canvas_free(&mut luma);
    canvas_free(&mut paper);
    // With ink, and an erase stroke that restores toward the picture under it.
    page_add(&mut p, line_stroke(TOOL_MARKER, 200.0, 5));
    page_add(&mut p, line_stroke(TOOL_PENCIL, 230.0, 6));
    let e = line_stroke(TOOL_ERASER, 215.0, erase_seed(TOOL_ERASER, 77));
    page_add(&mut p, e);
    check_pure(&p, "same size");
    // Drawn at another page size (a notebook an older version moved to a Scribe of the other size):
    // the picture is fitted to it, and still the same whichever way it is cut.
    let sizes: [[i32; 2]; 3] = [[643, 857], [600, 900], [700, 700]];
    for s in sizes {
        p.w = s[0];
        p.h = s[1];
        check_pure(&p, "another size");
    }
    // A 3:4 picture fills a 3:4 page of any size: no white band on either side.
    let mut c = Canvas::default();
    canvas_init(&mut c, 643, 857);
    canvas_fill(&mut c, rect(0, 0, 643, 857), 0);
    let mut bare = page_new(643, 857, p.paper);
    bare.nb.set(nb.id.as_str());
    let mut black = solid(PW, PH, 1, 0);
    put_picture(&nb, 0x000777, &black);
    bare.paper = paper_image(PAPER_ROLE_TEMPLATE, 0x000777);
    page_render(&mut c, &bare, rect(0, 0, 643, 857), DPI);
    let mut white = 0;
    for i in 0..643 * 857 {
        white += (c.px[i] == 255) as i32;
    }
    check!(white == 0);
    // Every page render took the paper from the picture, never from paper_render.
    check!(paper_take_image_blank_count() == 0);
    canvas_free(&mut c);
    page_free(bare);
    bgpicture_free(&mut black);
    page_free(p);
    bgpicture_free(&mut src);
    bgpicture_free(&mut fit);
}

fn missing_picture_draws_white() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Missing", "", PAPER_BLANK, &mut nb) == 0);
    let mut p = page_new(PW, PH, paper_image(PAPER_ROLE_TEMPLATE, 0xABCDEF));
    p.nb.set(nb.id.as_str());
    bgimage_take_missing();
    let mut c = Canvas::default();
    canvas_init(&mut c, PW, PH);
    canvas_fill(&mut c, rect(0, 0, PW, PH), 0);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    let mut white = 0;
    for i in 0..(PW * PH) as usize {
        white += (c.px[i] == 255) as i32;
    }
    check!(white == PW * PH);
    check!(bgimage_take_missing());
    check!(!bgimage_take_missing());
    // Remembered: the file arriving later changes nothing until the failures are forgotten.
    let mut g = solid(PW, PH, 1, 40);
    put_picture(&nb, 0xABCDEF, &g);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[(100 * PW + 100) as usize] == 255 && bgimage_take_missing());
    bgimage_retry_missing();
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[(100 * PW + 100) as usize] == 40 && !bgimage_take_missing());
    // An unreadable file draws white too; so does a page of no notebook.
    let (mut dir, mut file) = (String::new(), String::new());
    bgimage_dir(nb.id.as_str(), &mut dir, 700);
    bgimage_file(&dir, 0x0000AA, &mut file, 800);
    put_file(&file, b"not a picture");
    p.paper = paper_image(PAPER_ROLE_DOCUMENT, 0x0000AA);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[0] == 255 && c.px[(PW * PH - 1) as usize] == 255 && bgimage_take_missing());
    p.nb.clear();
    p.paper = paper_image(PAPER_ROLE_TEMPLATE, 0xABCDEF);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[(100 * PW + 100) as usize] == 255);
    canvas_free(&mut c);
    bgpicture_free(&mut g);
    page_free(p);
}

// ---- decoding and fitting ----

// A fixture picture decoded: no pixels when it is missing or does not decode.
fn load_fixture(name: &str, why: &mut &'static str) -> BgPicture {
    let path = fixture(&format!("pictures/{name}"));
    let d = slurp(&path);
    let mut p = BgPicture::default();
    *why = "no fixture";
    if let Some(d) = d {
        if bgimage_decode(&d, &mut p, why) != 0 {
            p.px = Vec::new();
        }
    }
    p
}

fn at(p: &BgPicture, x: i32, y: i32, c: i32) -> u8 {
    p.px[((y as usize * p.w as usize + x as usize) * p.ch as usize) + c as usize]
}

fn decoding() {
    let mut why: &'static str = "";
    // A 16-bit gray PNG: cut to 8 bits, a gradient from black at the top to white at the bottom.
    let mut g16 = load_fixture("gray16.png", &mut why);
    check!(!g16.px.is_empty() && g16.w == 60 && g16.h == 80 && g16.ch == 1);
    if !g16.px.is_empty() {
        check!(at(&g16, 30, 0, 0) < 10 && at(&g16, 30, 79, 0) > 245 && at(&g16, 30, 20, 0) < at(&g16, 30, 60, 0));
    }
    // A palette PNG with colour: RGB.
    let mut pal = load_fixture("palette.png", &mut why);
    check!(!pal.px.is_empty() && pal.ch == 3);
    if !pal.px.is_empty() {
        check!(at(&pal, 10, 10, 0) > 180 && at(&pal, 10, 10, 1) < 90 && at(&pal, 50, 70, 2) > 180);
    }
    // Transparency over white: the opaque black quarter stays black, transparent pixels turn white,
    // half-transparent red turns pink.
    let mut al = load_fixture("alpha.png", &mut why);
    check!(!al.px.is_empty() && al.ch == 3);
    if !al.px.is_empty() {
        check!(at(&al, 5, 5, 0) == 0 && at(&al, 5, 5, 1) == 0 && at(&al, 5, 5, 2) == 0);
        check!(at(&al, 50, 5, 0) == 255 && at(&al, 50, 5, 1) == 255 && at(&al, 50, 5, 2) == 255);
        check!(at(&al, 50, 70, 0) == 255 && at(&al, 50, 70, 1) > 110 && at(&al, 50, 70, 1) < 145);
    }
    // A progressive JPEG.
    let mut pj = load_fixture("progressive.jpg", &mut why);
    check!(!pj.px.is_empty() && pj.w == 60 && pj.h == 80 && pj.ch == 3);
    if !pj.px.is_empty() {
        check!(at(&pj, 30, 40, 1) > at(&pj, 30, 40, 0) && at(&pj, 30, 40, 1) > at(&pj, 30, 40, 2));
    }
    // PGM.
    let mut pgm = load_fixture("gray.pgm", &mut why);
    check!(!pgm.px.is_empty() && pgm.w == 30 && pgm.h == 40 && pgm.ch == 1);
    // Damage.
    let bad = load_fixture("broken.png", &mut why);
    check!(bad.px.is_empty());
    // An RGB picture whose pixels are all gray is kept gray.
    let mut rgb = solid(20, 30, 3, 90);
    for i in 0..20 * 30 {
        let v = (i % 251) as u8;
        rgb.px[3 * i] = v;
        rgb.px[3 * i + 1] = v;
        rgb.px[3 * i + 2] = v;
    }
    let path = format!("{}/grayrgb.png", root());
    put_png(&path, &rgb);
    let d = slurp(&path);
    let mut back = BgPicture::default();
    check!(d.as_ref().is_some_and(|d| bgimage_decode(d, &mut back, &mut why) == 0) && back.ch == 1 && back.px[123] == 123);
    // Luma, Rec. 709.
    let mut col = picture(3, 1, 3);
    let px: [u8; 9] = [255, 0, 0, 0, 255, 0, 0, 0, 255];
    col.px.copy_from_slice(&px);
    let mut l = Canvas::default();
    check!(bgimage_luma(&col, &mut l) == 0 && l.px[0] == 54 && l.px[1] == 182 && l.px[2] == 18);
    canvas_free(&mut l);
    bgpicture_free(&mut col);
    bgpicture_free(&mut g16);
    bgpicture_free(&mut pal);
    bgpicture_free(&mut al);
    bgpicture_free(&mut pj);
    bgpicture_free(&mut pgm);
    bgpicture_free(&mut rgb);
    bgpicture_free(&mut back);
}

// A PNG of w x h, `depth` bits a channel of colour type `color` (0 gray, 2 RGB, 6 RGBA), that is
// only its header (signature, IHDR, the start of an empty IDAT): enough for the size checks, which
// stop at the first IDAT.
fn png_header_of(out: &mut [u8], w: u32, h: u32, depth: u8, color: u8) -> usize {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    out[..8].copy_from_slice(&SIG);
    let ihdr: [u8; 25] = [
        0, 0, 0, 13, b'I', b'H', b'D', b'R', (w >> 24) as u8, (w >> 16) as u8, (w >> 8) as u8, w as u8, (h >> 24) as u8, (h >> 16) as u8,
        (h >> 8) as u8, h as u8, depth, color, 0, 0, 0, 0, 0, 0, 0,
    ];
    out[8..33].copy_from_slice(&ihdr);
    let idat: [u8; 12] = [0, 0, 0, 0, b'I', b'D', b'A', b'T', 0, 0, 0, 0];
    out[33..45].copy_from_slice(&idat);
    45
}

fn png_header(out: &mut [u8], w: u32, h: u32) -> usize {
    png_header_of(out, w, h, 8, 0)
}

fn oversized_refused() {
    let mut b = [0u8; 64];
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = "";
    let mut n = png_header(&mut b, 8000, 6000); // 48 megapixels
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) != 0 && why.contains("40 megapixels"));
    n = png_header(&mut b, 9000, 10); // past 8192 a side
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) != 0 && why.contains("8192"));
    n = png_header(&mut b, 6000, 6000); // 36 megapixels: taken
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) == 0 && w == 6000 && h == 6000);
    let mut p = BgPicture::default();
    n = png_header(&mut b, 8000, 6000);
    check!(bgimage_decode(&b[..n], &mut p, &mut why) != 0 && p.px.is_empty());
    // The memory it takes to decode, not only its pixels: 7900 x 5000 (39.5 MP) is taken in 8-bit
    // gray (about 80 MB), and refused as 16-bit RGBA (about 630 MB).
    n = png_header_of(&mut b, 7900, 5000, 8, 0);
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) == 0);
    let gray = bgimage_decode_cost(&b[..n], n);
    check!(gray > 79000000 && gray < 82000000);
    n = png_header_of(&mut b, 7900, 5000, 16, 6);
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) != 0 && why.contains("memory"));
    check!(bgimage_decode_cost(&b[..n], n) > (600usize << 20));
    check!(bgimage_decode(&b[..n], &mut p, &mut why) == -1 && p.px.is_empty() && why.contains("memory"));
    // 16-bit RGB: 12 bytes a pixel at its peak, so 20 MP is taken and 25 MP is not.
    n = png_header_of(&mut b, 5000, 4000, 16, 2);
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) == 0);
    n = png_header_of(&mut b, 5000, 5000, 16, 2);
    check!(bgimage_check(&b[..n], n, &mut w, &mut h, &mut why) != 0 && why.contains("memory"));
    // The file's own bytes count too: the same header as the start of a 250 MB file is refused.
    n = png_header_of(&mut b, 5000, 4000, 16, 2);
    check!(bgimage_check(&b[..n], 250usize << 20, &mut w, &mut h, &mut why) != 0);
}

// ---- the decode budget ----

struct Bits {
    d: Vec<u8>,
    acc: u32,
    bits: i32,
}

impl Bits {
    // deflate packs bits from the lowest up
    fn put_bits(&mut self, v: u32, n: i32) {
        for i in 0..n {
            self.acc |= ((v >> i) & 1) << self.bits;
            self.bits += 1;
            if self.bits == 8 {
                self.d.push(self.acc as u8);
                self.acc = 0;
                self.bits = 0;
            }
        }
    }

    // and a Huffman code from its top bit down
    fn put_code(&mut self, code: u32, n: i32) {
        for i in (0..n).rev() {
            self.put_bits((code >> i) & 1, 1);
        }
    }
}

// A 1x1 gray PNG whose IDAT inflates to `bytes` zeros (a "zlib bomb": the picture needs 2): a zlib
// stream of one block of fixed codes, a 0, then copies of 258 bytes from 1 back.
fn png_bomb(bytes: usize) -> Vec<u8> {
    let copies = bytes / 258;
    let mut hdr = [0u8; 64];
    let n = png_header_of(&mut hdr, 1, 1, 8, 0) - 12; // the signature and IHDR, no IDAT yet
    let mut b = Bits { d: Vec::new(), acc: 0, bits: 0 };
    b.d.push(0x78); // zlib: deflate, 32 KB window
    b.d.push(0x01);
    b.put_bits(1, 1); // the last block,
    b.put_bits(1, 2); // of fixed codes
    b.put_code(0x30, 8); // literal 0
    for _ in 0..copies {
        b.put_code(0xC5, 8); // length 258
        b.put_code(0, 5); // distance 1
    }
    b.put_code(0, 7); // the end of the block
    if b.bits != 0 {
        let rest = 8 - b.bits;
        b.put_bits(0, rest);
    }
    let bn = b.d.len() + 4; // its Adler-32 (left 0: never reached)
    let mut png = hdr[..n].to_vec();
    png.extend_from_slice(&(bn as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&b.d);
    png.extend_from_slice(&[0, 0, 0, 0]); // the Adler-32
    png.extend_from_slice(&[0, 0, 0, 0]); // (its CRC left 0: stb_image does not check it)
    const IEND: [u8; 12] = [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82];
    png.extend_from_slice(&IEND);
    png
}

fn decode_budget() {
    // A bomb: its header is a fine 1x1 picture, and decoding it stops at twice what that needs.
    let bomb = png_bomb(64usize << 20);
    let len = bomb.len();
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = "";
    check!(bgimage_check(&bomb, len, &mut w, &mut h, &mut why) == 0 && w == 1 && h == 1);
    let cost = bgimage_decode_cost(&bomb, len);
    let mut p = BgPicture::default();
    check!(bgimage_decode(&bomb, &mut p, &mut why) == -1 && p.px.is_empty() && why.contains("damaged"));
    check!(bgimage_last_decode_peak() <= 2 * cost + (1 << 20) && bgimage_last_decode_peak() < (4usize << 20));
    // Zeros after the picture's own data are allowed (stb_image reads what it needs), up to there.
    let bomb = png_bomb(16usize << 10);
    check!(bgimage_decode(&bomb, &mut p, &mut why) == 0 && p.w == 1 && p.h == 1 && !p.px.is_empty() && p.px[0] == 0);
    bgpicture_free(&mut p);
    // Every kind of picture takes no more to decode than its header says.
    let files = [
        "gray16.png",
        "gray16-big.png",
        "palette.png",
        "palette-trns.png",
        "alpha.png",
        "interlaced.png",
        "interlaced16.png",
        "progressive.jpg",
        "subsampled.jpg",
        "baseline.jpg",
        "gray.pgm",
        "gray16.pgm",
    ];
    for f in files {
        let path = fixture(&format!("pictures/{f}"));
        let d = slurp(&path);
        let c = d.as_ref().map_or(0, |d| bgimage_decode_cost(d, d.len()));
        let dec = d.as_ref().map_or(-1, |d| bgimage_decode(d, &mut p, &mut why));
        let peak = bgimage_last_decode_peak();
        if dec != 0 || peak == 0 || peak > c {
            eprintln!("  {f}: decode {dec}, peak {peak}, cost {c}");
        }
        check!(dec == 0 && peak > 0 && peak <= c);
        bgpicture_free(&mut p);
    }
    // Headers stb_image reads that are less plain: an iPhone PNG (its CgBI chunk before IHDR, its
    // data raw deflate) and a JPEG with padding between two segments. Taken, and decoded.
    {
        let mut g = noise(97, 61, 1);
        let path = format!("{}/plain.png", root());
        put_png(&path, &g);
        let png = slurp(&path);
        let len = png.as_ref().map_or(0, |p| p.len());
        let mut ip = vec![0u8; len + 64];
        let mut pos = 8usize;
        if let Some(png) = &png {
            ip[..8].copy_from_slice(&png[..8]);
        }
        let mut n = 8usize;
        const CGBI: [u8; 16] = [0, 0, 0, 4, b'C', b'g', b'B', b'I', 0x50, 0x00, 0x20, 0x02, 0, 0, 0, 0];
        ip[n..n + 16].copy_from_slice(&CGBI);
        n += 16;
        if let Some(png) = &png {
            // the chunks, the IDAT's zlib header and Adler-32 taken off
            while pos + 12 <= len {
                let cl = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
                if &png[pos + 4..pos + 8] == b"IDAT" {
                    let raw = cl - 6;
                    ip[n..n + 4].copy_from_slice(&(raw as u32).to_be_bytes());
                    ip[n + 4..n + 8].copy_from_slice(b"IDAT");
                    ip[n + 8..n + 8 + raw].copy_from_slice(&png[pos + 10..pos + 10 + raw]);
                    n += 12 + raw;
                } else {
                    ip[n..n + 12 + cl].copy_from_slice(&png[pos..pos + 12 + cl]);
                    n += 12 + cl;
                }
                pos += 12 + cl;
            }
        }
        check!(bgimage_check(&ip[..n], n, &mut w, &mut h, &mut why) == 0 && w == 97 && h == 61);
        let c = bgimage_decode_cost(&ip[..n], n);
        check!(bgimage_decode(&ip[..n], &mut p, &mut why) == 0 && !p.px.is_empty() && p.px[..97 * 61] == g.px[..97 * 61]);
        check!(bgimage_last_decode_peak() <= c);
        bgpicture_free(&mut p);
        bgpicture_free(&mut g);
        let jpg = slurp(&fixture("pictures/baseline.jpg")).unwrap_or_default();
        let len = jpg.len();
        let mut pad = vec![0u8; len + 8];
        let app0 = 4 + (((jpg[4] as usize) << 8) | jpg[5] as usize); // after SOI and APP0 (JFIF)
        pad[..app0].copy_from_slice(&jpg[..app0]);
        pad[app0..app0 + 3].fill(0);
        pad[app0 + 3..app0 + 3 + len - app0].copy_from_slice(&jpg[app0..]);
        let mut a = BgPicture::default();
        check!(jpg[2] == 0xFF && jpg[3] == 0xE0 && bgimage_check(&pad[..len + 3], len + 3, &mut w, &mut h, &mut why) == 0 && w == 900);
        check!(bgimage_decode(&jpg, &mut a, &mut why) == 0 && bgimage_decode(&pad[..len + 3], &mut p, &mut why) == 0);
        check!(
            !a.px.is_empty()
                && !p.px.is_empty()
                && a.ch == p.ch
                && a.px[..a.w as usize * a.h as usize * a.ch as usize] == p.px[..a.w as usize * a.h as usize * a.ch as usize]
        );
        bgpicture_free(&mut a);
        bgpicture_free(&mut p);
    }
    // A colour page picture as the store writes them (8 bits a channel), and a gray one.
    let mut ch = 1;
    while ch <= 3 {
        let mut big = noise(931, 1240, ch);
        let path = format!("{}/budget-{}.png", root(), ch);
        put_png(&path, &big);
        let d = slurp(&path);
        let c = d.as_ref().map_or(0, |d| bgimage_decode_cost(d, d.len()));
        check!(d.as_ref().is_some_and(|d| bgimage_decode(d, &mut p, &mut why) == 0) && bgimage_last_decode_peak() <= c);
        check!(!p.px.is_empty() && p.px[..931 * 1240 * ch as usize] == big.px[..931 * 1240 * ch as usize]);
        bgpicture_free(&mut p);
        bgpicture_free(&mut big);
        ch += 2;
    }
}

// What the UI thread decodes is a notebook's own picture: a file there that is no PNG, or bigger
// than a page picture can be, draws white (the editor is told), and is never decoded.
fn ui_picture_limits() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Odd", "", PAPER_BLANK, &mut nb) == 0);
    let (mut dir, mut file) = (String::new(), String::new());
    check!(bgimage_dir(nb.id.as_str(), &mut dir, 700) == 0);
    let mut p = page_new(PW, PH, PAPER_BLANK);
    p.nb.set(nb.id.as_str());
    let mut c = Canvas::default();
    canvas_init(&mut c, PW, PH);
    // A JPEG under a picture's name.
    let jpg = slurp(&fixture("pictures/baseline.jpg")).unwrap_or_default();
    bgimage_file(&dir, 0x0000B1, &mut file, 800);
    put_file(&file, &jpg);
    // A PNG of more than 32 MB (a picture, then an ancillary chunk of zeros).
    let mut g = solid(PW, PH, 1, 50);
    bgimage_file(&dir, 0x0000B2, &mut file, 800);
    padded_png(&file, &g, 33usize << 20);
    // One that takes more than 96 MB to decode (a 6000 x 5000 RGB header): fine for an import's
    // child, not here.
    let mut hdr = [0u8; 64];
    let hl = png_header_of(&mut hdr, 6000, 5000, 8, 2);
    let (mut w, mut h) = (0, 0);
    let mut why: &'static str = "";
    check!(bgimage_check(&hdr[..hl], hl, &mut w, &mut h, &mut why) == 0);
    bgimage_file(&dir, 0x0000B3, &mut file, 800);
    put_file(&file, &hdr[..hl]);
    let ids: [u32; 3] = [0x0000B1, 0x0000B2, 0x0000B3];
    let whys = ["is not a PNG", "is bigger than a page picture can be", "is too big to decode"];
    for i in 0..3 {
        p.paper = paper_image(PAPER_ROLE_TEMPLATE, ids[i]);
        canvas_fill(&mut c, rect(0, 0, PW, PH), 0);
        bgimage_take_missing();
        log_capture();
        page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
        let log = log_captured();
        check!(c.px[0] == 255 && c.px[((PH / 2) * PW + PW / 2) as usize] == 255 && bgimage_take_missing());
        check!(log.as_ref().is_some_and(|l| l.contains(whys[i]))); // (a file of 33 MB is not even read)
    }
    // The same picture as a PNG of its own size draws.
    bgimage_file(&dir, 0x0000B4, &mut file, 800);
    put_png(&file, &g);
    p.paper = paper_image(PAPER_ROLE_TEMPLATE, 0x0000B4);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[0] == 50 && !bgimage_take_missing());
    canvas_free(&mut c);
    page_free(p);
    bgpicture_free(&mut g);
}

// Rows [y0, y1) all v, columns [x0, x1).
fn block_is(p: &BgPicture, x0: i32, y0: i32, x1: i32, y1: i32, v: u8) -> bool {
    for y in y0..y1 {
        for x in x0..x1 {
            for c in 0..p.ch {
                if at(p, x, y, c) != v {
                    return false;
                }
            }
        }
    }
    true
}

fn normalization() {
    // Exact 3:4 (templates made for other tablets are often 1404x1872): it fills the page.
    let mut f = BgPicture::default();
    let mut s = solid(1404, 1872, 1, 0);
    check!(bgimage_normalize(&s, 1860, 2480, &mut f) == 0 && block_is(&f, 0, 0, 1860, 2480, 0));
    bgpicture_free(&mut s);
    bgpicture_free(&mut f);
    // Letter (8.5 x 11): as wide as the page, white bands above and below (2407 rows from row 36).
    s = solid(850, 1100, 1, 0);
    check!(bgimage_normalize(&s, 1860, 2480, &mut f) == 0);
    check!(block_is(&f, 0, 0, 1860, 36, 255) && block_is(&f, 0, 36, 1860, 36 + 2407, 0) && block_is(&f, 0, 36 + 2407, 1860, 2480, 255));
    bgpicture_free(&mut s);
    bgpicture_free(&mut f);
    // A4: as tall as the page, white bands left and right (1754 columns from column 53).
    s = solid(840, 1188, 3, 0);
    check!(bgimage_normalize(&s, 1860, 2480, &mut f) == 0 && f.ch == 3);
    check!(block_is(&f, 0, 0, 53, 2480, 255) && block_is(&f, 53, 0, 53 + 1754, 2480, 0) && block_is(&f, 53 + 1754, 0, 1860, 2480, 255));
    bgpicture_free(&mut s);
    bgpicture_free(&mut f);
    // A picture of the page's own size is kept as it is.
    s = noise(PW, PH, 1);
    check!(bgimage_normalize(&s, PW, PH, &mut f) == 0 && s.px[..(PW * PH) as usize] == f.px[..(PW * PH) as usize]);
    bgpicture_free(&mut f);
    // The same picture gives the same bytes every time (and so the same key).
    let mut f2 = BgPicture::default();
    check!(bgimage_normalize(&s, 1860, 2480, &mut f) == 0 && bgimage_normalize(&s, 1860, 2480, &mut f2) == 0);
    check!(f.px[..1860 * 2480] == f2.px[..1860 * 2480] && bgimage_key(&f) == bgimage_key(&f2));
    f2.px[999] ^= 1;
    check!(bgimage_key(&f) != bgimage_key(&f2));
    bgpicture_free(&mut s);
    bgpicture_free(&mut f);
    bgpicture_free(&mut f2);
    // Halving averages 2x2 blocks: blocks of one value stay that value exactly.
    s = picture(8, 8, 1);
    for y in 0..8 {
        for x in 0..8 {
            s.px[y * 8 + x] = (((x / 2) + (y / 2) * 4) * 16) as u8;
        }
    }
    check!(bgimage_normalize(&s, 4, 4, &mut f) == 0);
    let mut exact = true;
    for y in 0..4 {
        for x in 0..4 {
            exact = exact && f.px[y * 4 + x] == ((x + y * 4) * 16) as u8;
        }
    }
    check!(exact);
    bgpicture_free(&mut s);
    bgpicture_free(&mut f);
    // Scaled up (1404 -> 1860), a 1 px line stays a line, with its darkness: about 1.32 px of black,
    // spread over the two page pixels it covers.
    s = solid(1404, 1872, 1, 255);
    for y in 0..1872 {
        s.px[y * 1404 + 700] = 0;
    }
    check!(bgimage_normalize(&s, 1860, 2480, &mut f) == 0);
    let (mut darkest, mut ink) = (255, 0);
    for x in 920..935 {
        let v = f.px[1000 * 1860 + x] as i32;
        darkest = if v < darkest { v } else { darkest };
        ink += 255 - v;
    }
    check!(darkest < 100 && (ink - 255 * 1860 / 1404).abs() <= 3);
    bgpicture_free(&mut s);
    bgpicture_free(&mut f);
}

// ---- templates ----

fn template_file(name: &str, p: &BgPicture) -> String {
    let path = format!("{}/templates/{}", root(), name);
    put_png(&path, p);
    path
}

fn index_entries(nb: &NotebookInfo) -> i32 {
    let mut dir = String::new();
    bgimage_dir(nb.id.as_str(), &mut dir, 700);
    let mut ix = BgIndex::default();
    if bgindex_load(&dir, &mut ix) != 0 {
        return -1;
    }
    let n = ix.e.len() as i32;
    bgindex_free(&mut ix);
    n
}

fn templates_same_file_same_id() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Planner", "", PAPER_MEDIUM, &mut nb) == 0);
    let mut lines = solid(450, 600, 1, 255);
    let mut y = 40;
    while y < 600 {
        lines.px[y * 450..y * 450 + 450].fill(120);
        y += 30;
    }
    let path = template_file("Lines.png", &lines);
    let (mut k1, mut k2) = (PAPER_BLANK, PAPER_BLANK);
    let mut why: &'static str = "";
    check!(template_install(&nb, &path, &mut k1, &mut why) == 0);
    check!(paper_is_image(k1) && !paper_is_document(k1));
    // Its id: the low 24 bits of the file's key for this page size (no other picture here).
    let d = slurp(&path).unwrap_or_default();
    let want = (bgimage_source_key(&d, PW, PH) as u32) & 0xFFFFFF;
    check!(paper_image_id(k1) == (if want != 0 { want } else { 1 }));
    check!(template_install(&nb, &path, &mut k2, &mut why) == 0 && k2 == k1 && index_entries(&nb) == 1);
    let mut kind = -1;
    drain_jobs(Some(&mut kind));
    check!(kind == BgKind::Template as i32);
    // The picture is there now, made for the notebook's page size, and its page draws it.
    let (mut dir, mut file) = (String::new(), String::new());
    bgimage_dir(nb.id.as_str(), &mut dir, 700);
    bgimage_file(&dir, paper_image_id(k1), &mut file, 800);
    let d = slurp(&file);
    let mut made = BgPicture::default();
    check!(
        d.as_ref().is_some_and(|d| bgimage_decode(d, &mut made, &mut why) == 0)
            && made.w == PW
            && made.h == PH
            && made.ch == 1
    );
    let mut want_pic = BgPicture::default();
    check!(bgimage_normalize(&lines, PW, PH, &mut want_pic) == 0 && !made.px.is_empty() && made.px[..(PW * PH) as usize] == want_pic.px[..(PW * PH) as usize]);
    bgimage_retry_missing();
    let mut p = page_new(PW, PH, k1);
    p.nb.set(nb.id.as_str());
    let mut c = Canvas::default();
    canvas_init(&mut c, PW, PH);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[..(PW * PH) as usize] == want_pic.px[..(PW * PH) as usize]);
    // Another file: another paper.
    let mut dots = solid(300, 400, 1, 255);
    let mut y = 10;
    while y < 400 {
        let mut x = 10;
        while x < 300 {
            dots.px[y * 300 + x] = 0;
            x += 20;
        }
        y += 20;
    }
    let path2 = template_file("Dots.png", &dots);
    let mut k3 = PAPER_BLANK;
    check!(template_install(&nb, &path2, &mut k3, &mut why) == 0 && paper_is_image(k3) && k3 != k1 && index_entries(&nb) == 2);
    drain_jobs(None);
    // Picking it again restores a picture that went missing: same id, same bytes.
    unlink(&file);
    let mut k4 = PAPER_BLANK;
    check!(template_install(&nb, &path, &mut k4, &mut why) == 0 && k4 == k1);
    drain_jobs(None);
    let d = slurp(&file);
    let mut again = BgPicture::default();
    check!(
        d.as_ref().is_some_and(|d| bgimage_decode(d, &mut again, &mut why) == 0)
            && !again.px.is_empty()
            && again.px[..(PW * PH) as usize] == want_pic.px[..(PW * PH) as usize]
    );
    // Not a picture: refused at once, nothing queued.
    let junk = format!("{}/templates/junk.png", root());
    put_file(&junk, b"junk");
    check!(template_install(&nb, &junk, &mut k4, &mut why) != 0 && !bg_busy());
    unlink(&junk);
    canvas_free(&mut c);
    page_free(p);
    bgpicture_free(&mut lines);
    bgpicture_free(&mut dots);
    bgpicture_free(&mut made);
    bgpicture_free(&mut want_pic);
    bgpicture_free(&mut again);
}

// A file's hash read a piece at a time (as the UI may, for a small template) and from the whole
// file in memory (as the children do): the same, whatever the file's length.
fn source_key_streamed() {
    let sizes: [usize; 10] = [0, 1, 7, 8, 9, 65535, 65536, 65537, 131072, 200003];
    let mut d = vec![0u8; 200003];
    for v in d.iter_mut() {
        *v = rnd8();
    }
    let path = format!("{}/keyed.bin", root());
    for &sz in &sizes {
        put_file(&path, &d[..sz]);
        let mut k = 0u64;
        let mut len = 99usize;
        check!(bgimage_content_hash_file(&path, &mut k, &mut len) == 0 && len == sz);
        check!(k == bgimage_content_hash(&d[..sz]));
        let key = bgimage_template_key(k, len, PW, PH);
        check!(key == bgimage_source_key(&d[..sz], PW, PH));
        check!(key != bgimage_template_key(k, len, PH, PW)); // made for another page: another key
        if sz > 1 {
            check!(k != bgimage_content_hash(&d[1..sz]));
        }
    }
    check!(bgimage_content_hash_file("/nonexistent/x.png", &mut 0u64, &mut 0usize) != 0);
}

fn templates_collision_steps() {
    let (mut nb, mut nb2) = (NotebookInfo::default(), NotebookInfo::default());
    check!(store_create("Collide", "", PAPER_MEDIUM, &mut nb) == 0 && store_create("Orphan", "", PAPER_MEDIUM, &mut nb2) == 0);
    let mut g = noise(120, 160, 1);
    let path = template_file("Noise.png", &g);
    let d = slurp(&path).unwrap_or_default();
    let key = bgimage_source_key(&d, PW, PH);
    let mut id0 = (key as u32) & 0xFFFFFF;
    if id0 == 0 {
        id0 = 1;
    }
    let mut next = (id0 + 1) & 0xFFFFFF;
    if next == 0 {
        next = 1;
    }
    // Its id is taken by another key: the next one.
    let mut dir = String::new();
    bgimage_dir(nb.id.as_str(), &mut dir, 700);
    let text = cformat!("# taken\n%06x=%016llx\n", id0, key ^ 1);
    let ipath = format!("{dir}/index.ini");
    put_file(&ipath, text.as_bytes());
    let mut k = PAPER_BLANK;
    let mut why: &'static str = "";
    check!(template_install(&nb, &path, &mut k, &mut why) == 0 && paper_image_id(k) == next);
    check!(index_entries(&nb) == 2);
    // A picture file with no entry takes its id too, whatever it holds.
    let mut other = solid(PW, PH, 1, 9);
    put_picture(&nb2, id0, &other);
    let mut k2 = PAPER_BLANK;
    check!(template_install(&nb2, &path, &mut k2, &mut why) == 0 && paper_image_id(k2) == next);
    drain_jobs(None);
    // The squatter is never touched.
    let (mut d2, mut f2) = (String::new(), String::new());
    bgimage_dir(nb2.id.as_str(), &mut d2, 700);
    bgimage_file(&d2, id0, &mut f2, 800);
    let d = slurp(&f2);
    let mut back = BgPicture::default();
    check!(d.as_ref().is_some_and(|d| bgimage_decode(d, &mut back, &mut why) == 0) && !back.px.is_empty() && back.px[0] == 9);
    bgpicture_free(&mut back);
    // The children made each picture under the id its notebook gave it, the next one, and nothing
    // under the id that was taken; a page on it draws the template.
    let mut want = BgPicture::default();
    check!(bgimage_normalize(&g, PW, PH, &mut want) == 0);
    let both = [&nb, &nb2];
    let kinds = [k, k2];
    bgimage_retry_missing();
    for i in 0..2 {
        let (mut bd, mut fnm, mut f0) = (String::new(), String::new(), String::new());
        bgimage_dir(both[i].id.as_str(), &mut bd, 700);
        bgimage_file(&bd, next, &mut fnm, 800);
        bgimage_file(&bd, id0, &mut f0, 800);
        check!(exists(&fnm));
        if i == 0 {
            check!(!exists(&f0));
        }
        let mut pg = page_new(PW, PH, kinds[i]);
        pg.nb.set(both[i].id.as_str());
        let mut c = Canvas::default();
        canvas_init(&mut c, PW, PH);
        page_render(&mut c, &pg, rect(0, 0, PW, PH), DPI);
        check!(c.px[..(PW * PH) as usize] == want.px[..(PW * PH) as usize]);
        canvas_free(&mut c);
        page_free(pg);
    }
    bgpicture_free(&mut want);
    bgpicture_free(&mut g);
    bgpicture_free(&mut other);
    // bgindex_claim on its own: an existing key keeps its id, new keys step past taken ones.
    let mut ix = BgIndex::default();
    let (mut a, mut b, mut c) = (0u32, 0u32, 0u32);
    let mut added = false;
    check!(bgindex_claim(&mut ix, &dir, 0x0000000000000000, &mut a, &mut added) == 0 && a == 1 && added); // never 0
    check!(bgindex_claim(&mut ix, &dir, 0x1000000000000001, &mut b, &mut added) == 0 && b == 2 && added);
    check!(bgindex_claim(&mut ix, &dir, 0x0000000000000000, &mut c, &mut added) == 0 && c == 1 && !added);
    bgindex_free(&mut ix);
}

// One id in two notebooks names two pictures: the cache holds each by its notebook too, so
// pages drawn one after the other from either notebook each get their own.
fn same_id_two_notebooks() {
    let (mut a, mut b) = (NotebookInfo::default(), NotebookInfo::default());
    check!(store_create("Left", "", PAPER_BLANK, &mut a) == 0 && store_create("Right", "", PAPER_BLANK, &mut b) == 0);
    let (mut pa, mut pb) = (noise(PW, PH, 1), noise(PW, PH, 1));
    put_picture(&a, 0x00D00D, &pa);
    put_picture(&b, 0x00D00D, &pb);
    let mut qa = page_new(PW, PH, paper_image(PAPER_ROLE_DOCUMENT, 0x00D00D));
    let mut qb = page_new(PW, PH, qa.paper);
    qa.nb.set(a.id.as_str());
    qb.nb.set(b.id.as_str());
    let mut c = Canvas::default();
    canvas_init(&mut c, PW, PH);
    let mut right = true;
    for i in 0..6 {
        let q = if i % 2 != 0 { &qb } else { &qa };
        let want = if i % 2 != 0 { &pb } else { &pa };
        page_render(&mut c, q, rect(0, 0, PW, PH), DPI);
        right = right && c.px[..(PW * PH) as usize] == want.px[..(PW * PH) as usize];
        let rc = rect(37, 51, 211, 97); // a region alone, from the cache
        canvas_fill(&mut c, rect(0, 0, PW, PH), 0);
        page_render(&mut c, q, rc, DPI);
        let mut y = rc.y;
        while y < rc.y + rc.h && right {
            let (ca, wa) = ((y * c.stride + rc.x) as usize, (y as usize * PW as usize + rc.x as usize));
            right = c.px[ca..ca + rc.w as usize] == want.px[wa..wa + rc.w as usize];
            y += 1;
        }
    }
    check!(right);
    // Forgetting one notebook's pictures leaves the other's.
    bgimage_forget(Some(a.id.as_str()));
    page_render(&mut c, &qb, rect(0, 0, PW, PH), DPI);
    check!(c.px[..(PW * PH) as usize] == pb.px[..(PW * PH) as usize]);
    page_render(&mut c, &qa, rect(0, 0, PW, PH), DPI);
    check!(c.px[..(PW * PH) as usize] == pa.px[..(PW * PH) as usize]);
    check!(paper_take_image_blank_count() == 0);
    canvas_free(&mut c);
    page_free(qa);
    page_free(qb);
    bgpicture_free(&mut pa);
    bgpicture_free(&mut pb);
}

fn template_previews() {
    templates_init();
    let readme = format!("{}/templates/README.txt", root());
    check!(exists(&readme));
    let readme = format!("{}/import/README.txt", root());
    check!(exists(&readme));
    let mut a = solid(300, 400, 1, 200);
    let mut b = solid(300, 400, 3, 100);
    for i in 0..300 * 400 {
        b.px[3 * i] = 250;
    }
    template_file("t-10.png", &a);
    template_file("t-2.png", &b);
    let path = format!("{}/templates/t-3.jpg", root());
    put_file(&path, b"not a jpeg");
    let mut t: Vec<Template> = Vec::new();
    check!(template_scan(&mut t, DPI) == 0);
    let n = t.len();
    // Every picture file by name, in natural order, Lines and Dots and Noise from the tests above too.
    let (mut i2, mut i3, mut i10) = (-1i32, -1i32, -1i32);
    for i in 0..n {
        if t[i].name == "t-2" {
            i2 = i as i32;
        }
        if t[i].name == "t-3" {
            i3 = i as i32;
        }
        if t[i].name == "t-10" {
            i10 = i as i32;
        }
        check!(t[i].name != "README");
    }
    check!(i2 >= 0 && i2 < i3 && i3 < i10);
    if i10 >= 0 {
        let i = i10 as usize;
        check!(!t[i].ready && !t[i].bad && template_thumb(&mut t[i]).is_none());
    }
    template_list_free(&mut t);
    drain_jobs(None); // the previews job
    check!(template_scan(&mut t, DPI) == 0);
    for i in 0..t.len() {
        check!(t[i].thumb.px.is_empty()); // (none read by the scan)
        let pv = template_thumb(&mut t[i]).map(|c| (c.w, c.px.first().copied(), c.px.as_ptr()));
        if t[i].name == "t-3" {
            check!(t[i].bad && pv.is_none());
        }
        if t[i].name == "t-10" {
            let again = template_thumb(&mut t[i]).map(|c| c.px.as_ptr());
            check!(pv.is_some_and(|(w, px0, p)| w == PW / 6 && px0 == Some(200) && again == Some(p)));
        }
        if t[i].name == "t-2" {
            check!(pv.is_some() && !t[i].bad);
        }
    }
    template_list_free(&mut t);
    check!(!bg_busy()); // all made: nothing queued again
    bgpicture_free(&mut a);
    bgpicture_free(&mut b);
}

// A template's key without reading it whole on the UI thread: a big template is taken once the
// previews job has seen it (it keeps the hash of the file's bytes beside its preview), and the
// picture the BG_TEMPLATE child makes from the bytes is the one that key names.
fn big_template_uses_its_preview_key() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Big", "", PAPER_BLANK, &mut nb) == 0);
    let mut g = solid(300, 400, 1, 255);
    let mut y = 0;
    while y < 400 {
        g.px[y * 300..y * 300 + 300].fill(30);
        y += 25;
    }
    let path = format!("{}/templates/Big lines.png", root());
    padded_png(&path, &g, 5usize << 20); // past the 4 MB read on the UI thread
    check!(std::fs::metadata(&path).is_ok_and(|st| st.len() > (5 << 20)));
    let mut k = PAPER_BLANK;
    let mut why: &'static str = "";
    check!(template_install(&nb, &path, &mut k, &mut why) != 0 && why.contains("preview") && !bg_busy());
    let mut t: Vec<Template> = Vec::new();
    check!(template_scan(&mut t, DPI) == 0); // queues the previews job
    template_list_free(&mut t);
    drain_jobs(None);
    check!(template_install(&nb, &path, &mut k, &mut why) == 0 && paper_is_image(k));
    let d = slurp(&path);
    check!(d.as_ref().is_some_and(|d| paper_image_id(k) == ((bgimage_source_key(d, PW, PH) as u32) & 0xFFFFFF)));
    let mut kind = -1;
    drain_jobs(Some(&mut kind));
    check!(kind == BgKind::Template as i32);
    bgimage_retry_missing();
    let mut want = BgPicture::default();
    check!(bgimage_normalize(&g, PW, PH, &mut want) == 0);
    let mut p = page_new(PW, PH, k);
    p.nb.set(nb.id.as_str());
    let mut c = Canvas::default();
    canvas_init(&mut c, PW, PH);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    check!(c.px[..(PW * PH) as usize] == want.px[..(PW * PH) as usize]);
    canvas_free(&mut c);
    page_free(p);
    bgpicture_free(&mut want);
    bgpicture_free(&mut g);
    unlink(&path);
}

// ---- pages ----

fn document_role_not_inherited() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Score", "", PAPER_MUSIC, &mut nb) == 0);
    let doc = paper_image(PAPER_ROLE_DOCUMENT, 0x00BEEF);
    let tpl = paper_image(PAPER_ROLE_TEMPLATE, 0x00CAFE);
    check!(store_new_page_paper(&nb, doc) == PAPER_MUSIC);
    check!(store_new_page_paper(&nb, tpl) == tpl);
    check!(store_new_page_paper(&nb, PAPER_GRID) == PAPER_GRID);
    check!(store_add_page(&mut nb, 1, doc) == 0 && store_add_page(&mut nb, 2, tpl) == 0);
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 1, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == PAPER_MUSIC));
    if let Some(p) = p.take() {
        page_free(p);
    }
    check!(
        store_load_page(&nb, 2, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == tpl && p.nb.as_bytes() == nb.id.as_bytes())
    );
    if let Some(p) = p.take() {
        page_free(p);
    }
    // The added template page is in bg/pages.ini, the music one is not.
    let side = format!("{}/notebooks/{}/bg/pages.ini", root(), nb.id);
    let txt = slurp_text(&side);
    let want = cformat!("p%u=%u\n", nb.page_ids[2], tpl.0);
    check!(txt.as_ref().is_some_and(|t| t.contains(&want)));
    let want = cformat!("p%u=", nb.page_ids[1]);
    check!(txt.as_ref().is_some_and(|t| !t.contains(&want)));
    // A notebook whose own paper is a document page's: blank.
    nb.paper = doc;
    check!(store_new_page_paper(&nb, doc) == PAPER_BLANK);
}

// The page file as 1.0.0-beta saves it after opening it (a written page, a page delete).
fn beta_resaves(nb: &NotebookInfo, pid: u32) {
    let path = format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, pid);
    let d = slurp(&path);
    let mut b = Vec::new();
    check!(d.as_ref().is_some_and(|d| beta_resave(d, &mut b) == 0));
    if !b.is_empty() {
        put_file(&path, &b);
    }
}

fn file_version(nb: &NotebookInfo, pid: u32, paper16: Option<&mut u16>) -> u16 {
    let path = format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, pid);
    let d = slurp(&path);
    let ok = d.as_ref().is_some_and(|d| d.len() >= 8);
    let v = match &d {
        Some(d) if ok => (d[4] as u16) | ((d[5] as u16) << 8),
        _ => 0,
    };
    if let Some(p16) = paper16 {
        *p16 = match &d {
            Some(d) if ok => (d[6] as u16) | ((d[7] as u16) << 8),
            _ => 0,
        };
    }
    v
}

fn sidecar_restores_beta_resave() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Downgrade", "", PAPER_MEDIUM, &mut nb) == 0);
    check!(store_add_page(&mut nb, 1, PAPER_MEDIUM) == 0 && store_add_page(&mut nb, 2, PAPER_MEDIUM) == 0);
    let mut g = solid(PW, PH, 1, 77);
    put_picture(&nb, 0x0000C1, &g);
    let k = paper_image(PAPER_ROLE_TEMPLATE, 0x0000C1);
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 0, PW, PH, &mut p) == 0 && p.is_some());
    if let Some(pg) = p.as_deref_mut() {
        pg.paper = k;
        page_add(pg, line_stroke(TOOL_PENCIL, 300.0, 3));
        check!(store_save_page(&nb, 0, pg) == 0);
    }
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let side = format!("{}/notebooks/{}/bg/pages.ini", root(), nb.id);
    let mut txt = slurp_text(&side);
    let mut want = cformat!("p%u=%u\n", nb.page_ids[0], k.0);
    check!(txt.as_ref().is_some_and(|t| t.contains(&want)));
    // 1.0.0-beta re-saves the page: version 1 Blank, the stroke kept.
    beta_resaves(&nb, nb.page_ids[0]);
    let mut p16 = 0u16;
    check!(file_version(&nb, nb.page_ids[0], Some(&mut p16)) == 1 && p16 as u32 == PAPER_BLANK.0);
    // Back here, the page has its picture again. Exports read it so, changing nothing on disk.
    store_take_restored_count();
    check!(store_read_page(&nb, 0, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k && p.nb.as_bytes() == nb.id.as_bytes()));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let mut png = Buf::default();
    let mut info = PageRender::default();
    check!(export_page_png_ex(&nb, 0, DPI, PAGE_LAYER_PAPER, 1, Some(&mut info), &mut png) == 0 && info.paper == k.0);
    let mut back = BgPicture::default();
    let mut why: &'static str = "";
    check!(bgimage_decode(&png.data, &mut back, &mut why) == 0 && !back.px.is_empty() && block_is(&back, 0, 0, PW, PH, 77));
    bgpicture_free(&mut back);
    buf_free(&mut png);
    check!(file_version(&nb, nb.page_ids[0], None) == 1 && store_take_restored_count() == 0);
    // The editor's way in writes it back with its picture (version 2), once, and counts it: the
    // editor then redraws the thumbnail and the PDF the older version drew blank.
    check!(store_load_page(&nb, 0, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k && p.strokes.len() == 1));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    check!(file_version(&nb, nb.page_ids[0], Some(&mut p16)) == 2 && p16 as u32 == PAPER_BLANK.0 && store_take_restored_count() == 1);
    check!(store_load_page(&nb, 0, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k && p.strokes.len() == 1));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    check!(store_take_restored_count() == 0);
    // A list that does not read (no permission here; damage, a card error on the Kindle) is never
    // saved over: the entries it holds stay for when it reads again. (Root reads it anyway.)
    let mut before = String::new();
    txt = slurp_text(&side);
    if let Some(t) = &txt {
        strlcpy_q(&mut before, t, 256);
    }
    if geteuid() != 0 && chmod(&side, 0) == 0 {
        check!(store_load_page(&nb, 2, PW, PH, &mut p) == 0 && p.is_some());
        if let Some(pg) = p.as_deref_mut() {
            pg.paper = k;
            check!(store_save_page(&nb, 2, pg) == 0); // the page is saved; the list is left alone
        }
        if let Some(pg) = p.take() {
            page_free(pg);
        }
        check!(chmod(&side, 0o644) == 0);
        txt = slurp_text(&side);
        check!(txt.as_ref().is_some_and(|t| *t == before));
        check!(store_load_page(&nb, 2, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k)); // (its own file says so)
        if let Some(pg) = p.as_deref_mut() {
            pg.paper = PAPER_MEDIUM;
            check!(store_save_page(&nb, 2, pg) == 0);
        }
        if let Some(pg) = p.take() {
            page_free(pg);
        }
    }
    // A page set to Blank here leaves the list: it stays Blank.
    check!(store_load_page(&nb, 0, PW, PH, &mut p) == 0 && p.is_some());
    if let Some(pg) = p.as_deref_mut() {
        pg.paper = PAPER_BLANK;
        check!(store_save_page(&nb, 0, pg) == 0);
    }
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    txt = slurp_text(&side);
    check!(txt.as_ref().is_some_and(|t| !t.contains(&want)));
    check!(store_load_page(&nb, 0, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == PAPER_BLANK));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    // A page 1.0.0-beta gave another paper (its picker) keeps it.
    check!(store_load_page(&nb, 1, PW, PH, &mut p) == 0 && p.is_some());
    let mut d = Vec::new();
    if let Some(pg) = p.as_deref_mut() {
        pg.paper = k;
        check!(store_save_page(&nb, 1, pg) == 0);
        pg.paper = PAPER_GRID; // what 1.0.0-beta writes (version 1 Graph paper), the list left as it was
        check!(page_serialize(pg, &mut d) == 0);
    }
    let path = format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, nb.page_ids[1]);
    put_file(&path, &d);
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    check!(store_load_page(&nb, 1, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == PAPER_GRID));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    // A deleted page leaves the list.
    want = cformat!("p%u=", nb.page_ids[1]);
    txt = slurp_text(&side);
    check!(txt.as_ref().is_some_and(|t| t.contains(&want)));
    check!(store_delete_page(&mut nb, 1) == 0);
    txt = slurp_text(&side);
    check!(txt.as_ref().is_some_and(|t| !t.contains(&want)));
    bgpicture_free(&mut g);
}

fn beta_reads_image_pages() {
    let kinds = [paper_image(PAPER_ROLE_TEMPLATE, 0x00F00D), paper_image(PAPER_ROLE_DOCUMENT, 0xFFFFFF)];
    for kind in kinds {
        let mut p = page_new(1860, 2480, kind);
        page_add(&mut p, line_stroke(TOOL_BALLPOINT, 400.0, 9));
        page_add(&mut p, line_stroke(TOOL_ERASER, 400.0, erase_seed(TOOL_PENCIL, 4)));
        let mut d = Vec::new();
        check!(page_serialize(&p, &mut d) == 0 && d[4] == 2);
        let mut bp = BetaPage::default();
        check!(beta_page_deserialize(&d, &mut bp) == 0);
        check!(bp.paper == BETA_PAPER_BLANK && bp.n == 2 && bp.strokes[0].n == 30 && bp.strokes[1].seed == p.strokes[1].seed);
        beta_page_free(&mut bp);
        let mut b2 = Vec::new();
        check!(beta_resave(&d, &mut b2) == 0 && !b2.is_empty() && b2[4] == 1 && b2[6] as u32 == BETA_PAPER_BLANK);
        let mut q: Option<Box<Page>> = None;
        check!(page_deserialize(&b2, &mut q) == 0 && q.as_ref().is_some_and(|q| q.paper == PAPER_BLANK && q.strokes.len() == 2)); // (the store restores it)
        if let Some(q) = q {
            page_free(q);
        }
        page_free(p);
    }
}

// Erasing over a picture restores the picture, and compaction keeps the page as it looks.
fn erasing_over_a_picture() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Erase", "", PAPER_BLANK, &mut nb) == 0);
    let mut n = noise(PW, PH, 1);
    put_picture(&nb, 0x0000E1, &n);
    let mut p = page_new(PW, PH, paper_image(PAPER_ROLE_DOCUMENT, 0x0000E1));
    p.nb.set(nb.id.as_str());
    page_add(&mut p, line_stroke(TOOL_MARKER, 300.0, 1));
    let mut big = ToolParams::default();
    tool_params_default(TOOL_ERASER, &mut big);
    big.size_min_mm = 12.0;
    big.size_max_mm = 12.0;
    let mut e = stroke_new(TOOL_ERASER, &big, erase_seed(TOOL_ERASER, 2));
    for i in 0..60 {
        let q = PenSample { x: fmla(i as f32, 10.0, 20.0), y: 310.0, pressure: 1.0, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 };
        stroke_append(&mut e, &q, DPI);
    }
    page_add(&mut p, e);
    let (mut c, mut bare) = (Canvas::default(), Canvas::default());
    canvas_init(&mut c, PW, PH);
    canvas_init(&mut bare, PW, PH);
    page_render(&mut c, &p, rect(0, 0, PW, PH), DPI);
    page_paper_render(&mut bare, &p, rect(0, 0, PW, PH), DPI);
    // Under the eraser's middle the picture shows again, not white.
    let (mut restored, mut white) = (0, 0);
    for x in 100..500usize {
        let i = 310 * PW as usize + x;
        restored += (c.px[i] == bare.px[i]) as i32;
        white += (c.px[i] == 255 && bare.px[i] != 255) as i32;
    }
    check!(restored > 390 && white == 0);
    let mut drop = vec![false; p.strokes.len()];
    let nd = page_droppable(&p, DPI, &mut drop);
    check!(nd >= 0);
    let mut q = page_new(PW, PH, p.paper);
    q.nb.set(nb.id.as_str());
    for i in 0..p.strokes.len() {
        if !drop[i] {
            let src = &p.strokes[i];
            let mut s = stroke_new(src.tool, &src.params, src.seed);
            for pt in &src.pts {
                stroke_append(&mut s, pt, DPI);
            }
            page_add(&mut q, s);
        }
    }
    let mut c2 = Canvas::default();
    canvas_init(&mut c2, PW, PH);
    page_render(&mut c2, &q, rect(0, 0, PW, PH), DPI);
    check!(c.px[..(PW * PH) as usize] == c2.px[..(PW * PH) as usize]);
    check!(paper_take_image_blank_count() == 0); // (compaction's paper too)
    canvas_free(&mut c);
    canvas_free(&mut c2);
    canvas_free(&mut bare);
    page_free(p);
    page_free(q);
    bgpicture_free(&mut n);
}

// Every page bg/pages.ini lists that an older version saved on blank paper gets its picture back as
// the notebook opens in the editor, also the pages nobody visits: the notebook's thumbnail and PDF,
// which the older version drew blank, are drawn again when it closes (store_take_restored_count).
fn listed_pages_restored_at_open() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Restored", "", PAPER_MEDIUM, &mut nb) == 0);
    for i in 1..4 {
        check!(store_add_page(&mut nb, i, PAPER_MEDIUM) == 0);
    }
    let mut g = solid(PW, PH, 1, 90);
    put_picture(&nb, 0x0000D1, &g);
    let k = paper_image(PAPER_ROLE_DOCUMENT, 0x0000D1);
    for i in 1..4 {
        // pages 2 to 4 on the picture, page 2 written on
        let mut p: Option<Box<Page>> = None;
        check!(store_load_page(&nb, i, PW, PH, &mut p) == 0 && p.is_some());
        if let Some(pg) = p.as_deref_mut() {
            pg.paper = k;
            if i == 1 {
                page_add(pg, line_stroke(TOOL_PENCIL, 200.0, 5));
            }
            check!(store_save_page(&nb, i, pg) == 0);
        }
        if let Some(pg) = p {
            page_free(pg);
        }
    }
    // 1.0.0-beta opened pages 2 and 3 and saved them (version 1 Blank); page 4 it left alone.
    beta_resaves(&nb, nb.page_ids[1]);
    beta_resaves(&nb, nb.page_ids[2]);
    // Listed too: a page no longer in the notebook (its file there all the same), and a read-only
    // page (a chunk this version does not know) that reads as Blank. Both stay as they are.
    let gone = nb.next_page_id + 7;
    let ro = nb.page_ids[3];
    let b = page_new(PW, PH, PAPER_BLANK);
    let mut d = Vec::new();
    check!(page_serialize(&b, &mut d) == 0);
    let dl = d.len();
    let mut path = format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, gone);
    put_file(&path, &d);
    let mut crit = d.clone();
    crit.extend_from_slice(b"ABCD\0\0\0\0");
    path = format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, ro);
    put_file(&path, &crit);
    page_free(b);
    let side = format!("{}/notebooks/{}/bg/pages.ini", root(), nb.id);
    let txt = slurp(&side);
    let line = cformat!("p%u=%u\n", gone, k.0);
    let mut more = txt.unwrap_or_default();
    more.extend_from_slice(line.as_bytes());
    put_file(&side, &more);
    store_take_restored_count();
    check!(store_restore_listed_pages(&nb) == 2 && store_take_restored_count() == 2);
    let mut p16 = 0u16;
    check!(file_version(&nb, nb.page_ids[1], Some(&mut p16)) == 2 && p16 as u32 == PAPER_BLANK.0 && file_version(&nb, nb.page_ids[2], None) == 2);
    let mut p: Option<Box<Page>> = None;
    check!(store_read_page(&nb, 1, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k && p.strokes.len() == 1)); // (its ink kept)
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let rod = slurp(&path);
    check!(rod.as_ref().is_some_and(|rod| rod.len() == dl + 8 && *rod == crit)); // the read-only page as it was
    check!(file_version(&nb, gone, None) == 1);
    // Nothing left to bring back: a second opening writes nothing.
    check!(store_restore_listed_pages(&nb) == 0 && store_take_restored_count() == 0);
    bgpicture_free(&mut g);
}

// A notebook made on a template (New notebook's More papers...): its own paper and its first page
// are the template's image paper, which notebook.ini keeps and new pages get; 1.0.0-beta reads that
// paper as Medium rule (its pages keep theirs, and bg/pages.ini lists the first).
fn notebook_on_a_template() {
    let mut nb = NotebookInfo::default();
    check!(store_create("On a template", "", PAPER_GRID, &mut nb) == 0);
    let mut lines = solid(300, 400, 1, 255);
    let mut y = 20;
    while y < 400 {
        lines.px[y * 300..y * 300 + 300].fill(140);
        y += 25;
    }
    let path = template_file("Planner week.png", &lines);
    let mut why: &'static str = "";
    check!(template_start_notebook(&mut nb, &path, &mut why) == 0 && paper_is_image(nb.paper));
    let k = nb.paper;
    let mut back = NotebookInfo::default();
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.paper == k);
    check!(beta_paper_sanitize(k.0, 1) == 1); // (1.0.0-beta: Medium rule)
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 0, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let side = format!("{}/notebooks/{}/bg/pages.ini", root(), nb.id);
    let txt = slurp_text(&side);
    let want = cformat!("p%u=%u\n", nb.page_ids[0], k.0);
    check!(txt.as_ref().is_some_and(|t| t.contains(&want)));
    // The next page gets it, and so does a page whose file is missing (the notebook's paper).
    check!(store_add_page(&mut nb, 1, k) == 0);
    check!(store_load_page(&nb, 1, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let path = format!("{}/notebooks/{}/p{}.qpg", root(), nb.id, nb.page_ids[1]);
    unlink(&path);
    check!(store_read_page(&nb, 1, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == k));
    // Its picture, once the background has made it, is what the page draws.
    drain_jobs(None);
    bgimage_retry_missing();
    let mut want_pic = BgPicture::default();
    check!(bgimage_normalize(&lines, PW, PH, &mut want_pic) == 0);
    let mut c = Canvas::default();
    canvas_init(&mut c, PW, PH);
    if let Some(pg) = p.as_deref() {
        page_render(&mut c, pg, rect(0, 0, PW, PH), DPI);
    }
    check!(c.px[..(PW * PH) as usize] == want_pic.px[..(PW * PH) as usize] && !bgimage_take_missing());
    canvas_free(&mut c);
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    bgpicture_free(&mut want_pic);
    bgpicture_free(&mut lines);
}

// The notebook's card drawn on white paper (the editor left the notebook before the template's
// picture was made): the job that makes the picture draws the card again when the first page is on
// it, and leaves it alone when the first page is not.
fn card_drawn_again_with_its_picture() {
    let mut g = solid(300, 400, 1, 255);
    let mut y = 30;
    while y < 400 {
        g.px[y * 300..y * 300 + 300].fill(60);
        y += 40;
    }
    let path = template_file("Card lines.png", &g);
    let (mut on, mut off) = (NotebookInfo::default(), NotebookInfo::default());
    check!(store_create("Card", "", PAPER_MEDIUM, &mut on) == 0 && store_create("Card 2", "", PAPER_MEDIUM, &mut off) == 0);
    let (mut k, mut k2) = (PAPER_BLANK, PAPER_BLANK);
    let mut why: &'static str = "";
    check!(template_install(&on, &path, &mut k, &mut why) == 0 && store_start_on(&mut on, k) == 0);
    check!(store_add_page(&mut off, 1, PAPER_MEDIUM) == 0 && template_install(&off, &path, &mut k2, &mut why) == 0);
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&off, 1, PW, PH, &mut p) == 0 && p.is_some());
    if let Some(pg) = p.as_deref_mut() {
        pg.paper = k2; // (the second page of the other notebook)
        check!(store_save_page(&off, 1, pg) == 0);
    }
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let mut white = Canvas::default();
    canvas_init(&mut white, PW / 3, PH / 3);
    check!(store_save_thumb(on.id.as_str(), &white) == 0 && store_save_thumb(off.id.as_str(), &white) == 0);
    drain_jobs(None);
    bgimage_retry_missing();
    let (mut card, mut want, mut other) = (Canvas::default(), Canvas::default(), Canvas::default());
    check!(store_load_thumb(on.id.as_str(), &mut card) == 0 && store_read_page(&on, 0, &mut p) == 0 && p.is_some());
    canvas_init(&mut want, PW / 3, PH / 3);
    if let Some(pg) = p.take() {
        page_render_thumb(&mut want, &pg, DPI);
        page_free(pg);
    }
    let wh = (want.w * want.h) as usize;
    check!(card.w == want.w && card.h == want.h && card.px[..wh] == want.px[..wh]);
    let wwh = (white.w * white.h) as usize;
    check!(card.px[..wwh] != white.px[..wwh]);
    check!(store_load_thumb(off.id.as_str(), &mut other) == 0 && other.px[..wwh] == white.px[..wwh]);
    canvas_free(&mut card);
    canvas_free(&mut want);
    canvas_free(&mut other);
    canvas_free(&mut white);
    unlink(&path);
    bgpicture_free(&mut g);
}

// The editor writing the notebook's card while the job draws it (leaving page 1, which it saves
// first): the job draws the card again from page 1 as it is then, and writes that, never a mix of
// the two; one that keeps changing is left as the editor wrote it after three tries. No file of
// the job's is left behind.
static G_MEANWHILE: Mutex<Option<NotebookInfo>> = Mutex::new(None);
static G_MEANWHILE_WRITES: AtomicI32 = AtomicI32::new(0);
static G_MEANWHILE_MAX: AtomicI32 = AtomicI32::new(0);

// (in the job's child)
fn editor_meanwhile(id: &str) {
    let nb = G_MEANWHILE.lock().unwrap().clone().unwrap_or_default();
    if id != nb.id.as_str() || G_MEANWHILE_WRITES.load(Ordering::Relaxed) >= G_MEANWHILE_MAX.load(Ordering::Relaxed) {
        return;
    }
    let writes = G_MEANWHILE_WRITES.fetch_add(1, Ordering::Relaxed) + 1;
    let mut p: Option<Box<Page>> = None;
    if store_load_page(&nb, 0, PW, PH, &mut p) == 0 {
        if let Some(mut pg) = p {
            page_add(&mut pg, line_stroke(TOOL_PENCIL, 100.0 + 60.0 * writes as f32, 40 + writes as u32));
            store_save_page(&nb, 0, &pg);
            page_free(pg);
        }
    }
    let mut t = Canvas::default();
    if canvas_init(&mut t, PW / 3, PH / 3) == 0 {
        let (w, h) = (t.w, t.h);
        canvas_fill(&mut t, rect(0, 0, w, h), (10 * writes) as u8);
        store_save_thumb(id, &t);
        canvas_free(&mut t);
    }
}

fn card_written_meanwhile() {
    let mut g = solid(300, 400, 1, 255);
    let mut y = 30;
    while y < 400 {
        g.px[y * 300..y * 300 + 300].fill(60);
        y += 40;
    }
    for round in 0..2 {
        let name = format!("Meanwhile {round}.png");
        g.px[round] = 0; // (another picture each round: its own job)
        let path = template_file(&name, &g);
        let mut nb = NotebookInfo::default();
        check!(store_create("Meanwhile", "", PAPER_MEDIUM, &mut nb) == 0);
        *G_MEANWHILE.lock().unwrap() = Some(nb.clone());
        let mut white = Canvas::default();
        canvas_init(&mut white, PW / 3, PH / 3);
        check!(store_save_thumb(nb.id.as_str(), &white) == 0); // (the editor's, on white paper)
        canvas_free(&mut white);
        // (All set before the job's child is forked, as the template goes in.)
        G_MEANWHILE_WRITES.store(0, Ordering::Relaxed);
        G_MEANWHILE_MAX.store(if round != 0 { 5 } else { 1 }, Ordering::Relaxed); // once; then every time
        *STORE_TEST_BEFORE_CARD_REPLACE.lock().unwrap() = Some(editor_meanwhile);
        log_capture();
        let mut why: &'static str = "";
        check!(template_start_notebook(&mut nb, &path, &mut why) == 0);
        drain_jobs(None);
        let log = log_captured();
        *STORE_TEST_BEFORE_CARD_REPLACE.lock().unwrap() = None;
        bgimage_retry_missing();
        let (mut card, mut want) = (Canvas::default(), Canvas::default());
        let mut p: Option<Box<Page>> = None;
        check!(store_load_thumb(nb.id.as_str(), &mut card) == 0 && store_read_page(&nb, 0, &mut p) == 0 && p.is_some());
        if let Some(pg) = p.as_deref() {
            if round == 0 {
                // Drawn again from page 1 with the editor's stroke, over the editor's card.
                check!(pg.strokes.len() == 1);
                canvas_init(&mut want, PW / 3, PH / 3);
                page_render_thumb(&mut want, pg, DPI);
                let wh = (want.w * want.h) as usize;
                check!(card.w == want.w && card.h == want.h && card.px[..wh] == want.px[..wh]);
                canvas_free(&mut want);
            } else {
                // The editor wrote one each time (three): its last stays, whole.
                check!(pg.strokes.len() == 3 && log.as_ref().is_some_and(|l| l.contains("changed each time")));
                let mut all = !card.px.is_empty();
                let mut i = 0;
                while all && i < card.w * card.h {
                    all = card.px[((i / card.w) * card.stride + (i % card.w)) as usize] == 30;
                    i += 1;
                }
                check!(all);
            }
        }
        if let Some(pg) = p.take() {
            page_free(pg);
        }
        canvas_free(&mut card);
        let dir = format!("{}/notebooks/{}", root(), nb.id);
        check!(count_prefix(&dir, "thumb.pgm.") == 0);
        unlink(&path);
    }
    bgpicture_free(&mut g);
}

// The pieces of it: thumb.pgm's bytes, written over only while they are what was read, and the
// stamp a card read from it keeps (store_thumb_stamp), which another write of it changes.
fn card_replaced_only_if_same() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Card file", "", PAPER_MEDIUM, &mut nb) == 0);
    let id = nb.id.as_str();
    let mut was = store_read_thumb_file(id);
    check!(was.is_none() && store_thumb_stamp(id) == 0);
    let (mut a, mut b, mut got) = (Canvas::default(), Canvas::default(), Canvas::default());
    canvas_init(&mut a, 40, 50);
    canvas_init(&mut b, 40, 50);
    canvas_fill(&mut a, rect(0, 0, 40, 50), 30);
    canvas_fill(&mut b, rect(0, 0, 40, 50), 200);
    // None there, none read: written.
    check!(store_replace_thumb_if_same(id, &a, &[]) == 0);
    check!(store_load_thumb(id, &mut got) == 0 && got.px[0] == 30);
    canvas_free(&mut got);
    let st = store_thumb_stamp(id);
    check!(st != 0 && (st & 1) != 0 && st == store_thumb_stamp(id));
    // Read, then written again by another (the editor): kept as that one wrote it.
    was = store_read_thumb_file(id);
    check!(was.as_ref().is_some_and(|w| !w.is_empty()));
    check!(store_save_thumb(id, &b) == 0 && store_thumb_stamp(id) != st);
    check!(store_replace_thumb_if_same(id, &a, was.as_deref().unwrap_or(&[])) == 1);
    check!(store_load_thumb(id, &mut got) == 0 && got.px[0] == 200);
    canvas_free(&mut got);
    check!(store_replace_thumb_if_same(id, &a, &[]) == 1); // (one is there, none was read)
    // Read as it is now: written.
    was = store_read_thumb_file(id);
    check!(store_replace_thumb_if_same(id, &a, was.as_deref().unwrap_or(&[])) == 0);
    check!(store_load_thumb(id, &mut got) == 0 && got.px[0] == 30);
    canvas_free(&mut got);
    let dir = format!("{}/notebooks/{}", root(), nb.id);
    check!(count_prefix(&dir, "thumb.pgm.") == 0);
    canvas_free(&mut a);
    canvas_free(&mut b);
}

// New notebook on a template that cannot be used (a file that is no picture): the notebook stays
// on the paper it was made on, its first page too, and nothing is put into it.
fn new_notebook_template_refused() {
    let path = format!("{}/templates/Not a picture.png", root());
    put_file(&path, b"no picture");
    let mut nb = NotebookInfo::default();
    check!(store_create("Refused", "", PAPER_WIDE, &mut nb) == 0);
    let mut why: &'static str = "";
    check!(template_start_notebook(&mut nb, &path, &mut why) == -1 && !why.is_empty() && nb.paper == PAPER_WIDE && !bg_busy());
    let mut back = NotebookInfo::default();
    let mut p: Option<Box<Page>> = None;
    check!(store_load_info(nb.id.as_str(), &mut back) == 0 && back.paper == PAPER_WIDE);
    check!(store_read_page(&back, 0, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == PAPER_WIDE));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    let mut bgdir = String::new();
    check!(bgimage_dir(nb.id.as_str(), &mut bgdir, 700) == 0 && !exists(&bgdir));
    unlink(&path);
}

// bg/index.ini there but not readable (a card error, a permission): it is never saved over, which
// would lose the ids it names, so no template goes into the notebook until it reads again, neither
// on the UI thread nor in the background job. (Root reads it anyway.)
fn unreadable_index_kept() {
    if geteuid() == 0 {
        return;
    }
    let mut nb = NotebookInfo::default();
    check!(store_create("Index", "", PAPER_MEDIUM, &mut nb) == 0);
    let (mut a, mut b) = (solid(300, 400, 1, 210), solid(300, 400, 1, 120));
    let pa = template_file("Index a.png", &a);
    let pb = template_file("Index b.png", &b);
    let (mut ka, mut kb, mut again) = (PAPER_BLANK, PAPER_BLANK, PAPER_BLANK);
    let mut why: &'static str = "";
    check!(template_install(&nb, &pa, &mut ka, &mut why) == 0);
    drain_jobs(None);
    let mut bgdir = String::new();
    check!(bgimage_dir(nb.id.as_str(), &mut bgdir, 700) == 0);
    let ix = format!("{bgdir}/index.ini");
    let before = slurp_text(&ix);
    check!(before.as_ref().is_some_and(|b| b.contains('=')) && chmod(&ix, 0) == 0);
    let mut idx = BgIndex::default();
    check!(bgindex_load(&bgdir, &mut idx) == -1 && idx.e.is_empty());
    check!(template_install(&nb, &pb, &mut kb, &mut why) == -1 && why.contains("cannot be written") && !bg_busy());
    check!(template_install(&nb, &pa, &mut again, &mut why) == -1); // (even the one it names)
    check!(bg_template(nb.id.as_str(), &pb, DPI) != 0); // the job's side refuses it too
    check!(chmod(&ix, 0o644) == 0);
    let after = slurp_text(&ix);
    check!(after.is_some() && before == after);
    // Readable again: the first keeps its paper, the second gets one of its own.
    check!(template_install(&nb, &pa, &mut again, &mut why) == 0 && again == ka);
    check!(template_install(&nb, &pb, &mut kb, &mut why) == 0 && kb != ka && index_entries(&nb) == 2);
    drain_jobs(None);
    unlink(&pa);
    unlink(&pb);
    bgpicture_free(&mut a);
    bgpicture_free(&mut b);
}

// A template that does not read when the previews are made (no memory for it, a read error; here
// no permission) is not marked bad: the next scan tries it again, and once it reads it is offered.
fn unreadable_template_tried_later() {
    if geteuid() == 0 {
        return;
    }
    let mut g = solid(300, 400, 1, 180);
    let path = template_file("Later.png", &g);
    check!(chmod(&path, 0) == 0);
    let mut t: Vec<Template> = Vec::new();
    check!(template_scan(&mut t, DPI) == 0);
    template_list_free(&mut t);
    drain_jobs(None);
    for pass in 0..2 {
        check!(template_scan(&mut t, DPI) == 0);
        let mut i: i32 = -1;
        for k in 0..t.len() {
            if t[k].name == "Later" {
                i = k as i32;
            }
        }
        if pass == 0 {
            check!(i >= 0 && !t[i as usize].ready && !t[i as usize].bad && bg_busy()); // (asked for again)
        } else {
            check!(i >= 0 && t[i as usize].ready && !t[i as usize].bad);
        }
        template_list_free(&mut t);
        if pass == 0 {
            drain_jobs(None);
            check!(chmod(&path, 0o644) == 0);
            check!(template_scan(&mut t, DPI) == 0); // its preview made now
            template_list_free(&mut t);
            drain_jobs(None);
        }
    }
    unlink(&path);
    bgpicture_free(&mut g);
}

// ---- imports ----

fn natural_order() {
    check!(natural_cmp("page-2", "page-10") < 0 && natural_cmp("page-10", "page-2") > 0);
    check!(natural_cmp("page-9.png", "page-10.png") < 0);
    check!(natural_cmp("Page-3", "page-4") < 0 && natural_cmp("a", "B") < 0);
    check!(natural_cmp("page-002", "page-3") < 0 && natural_cmp("page-02", "page-2") != 0);
    check!(natural_cmp("x", "x1") < 0 && natural_cmp("x1y2", "x1y10") < 0 && natural_cmp("same", "same") == 0);
    check!(natural_cmp("scan 1 of 12", "scan 12 of 12") < 0);
    check!(natural_cmp("100", "99") > 0 && natural_cmp("0", "00") != 0);
}

fn import_folder_of(name: &str, files: &[&str], values: &[u8], n: usize) -> String {
    let src = format!("{}/import/{}", root(), name);
    for i in 0..n {
        let path = format!("{}/{}", src, files[i]);
        if values[i] == 1 {
            put_file(&path, b"broken");
            continue;
        }
        let mut p = solid(150, 200, 1, values[i]);
        put_png(&path, &p);
        bgpicture_free(&mut p);
    }
    src
}

// The mark import_scan leaves in a folder: the notebook it becomes.
fn mark(src: &str, id: &str) {
    let path = format!("{src}/.grimoire-import");
    let text = format!("{id}\n");
    put_file(&path, text.as_bytes());
}

fn page_value(nb: &NotebookInfo, index: i32) -> u8 {
    let mut p: Option<Box<Page>> = None;
    let mut v = 0;
    if store_read_page(nb, index, &mut p) == 0 {
        if let Some(pg) = p.as_deref() {
            let mut c = Canvas::default();
            canvas_init(&mut c, pg.w, pg.h);
            page_render(&mut c, pg, rect(0, 0, pg.w, pg.h), DPI);
            v = c.px[((pg.h / 2) * c.stride + pg.w / 2) as usize];
            canvas_free(&mut c);
        }
    }
    if let Some(pg) = p {
        page_free(pg);
    }
    v
}

fn import_makes_a_notebook() {
    let files = ["page-10.png", "page-2.png", "page-1.png", "notes.txt", "page-3.PNG"];
    let values: [u8; 5] = [90, 60, 30, 0, 70];
    let src = import_folder_of("Score", &files, &values, 3);
    let mut path = format!("{src}/notes.txt");
    put_file(&path, b"notes");
    path = format!("{src}/page-3.PNG");
    let mut p3 = solid(150, 200, 3, 70);
    p3.px[0] = 200; // one coloured pixel: kept in colour
    put_png(&path, &p3);
    bgpicture_free(&mut p3);
    let id = "20261020-120000-abcd";
    mark(&src, id);
    check!(bg_import_folder(&src, DPI) == 0);
    let mut nb = NotebookInfo::default();
    check!(store_load_info(id, &mut nb) == 0);
    check!(nb.name == "Score" && nb.folder.is_empty() && nb.npages() == 4 && nb.paper == PAPER_BLANK);
    check!(nb.page_w == PW && nb.page_h == PH && nb.next_page_id == 5);
    // Natural order: page-1, page-2, page-3, page-10.
    check!(page_value(&nb, 0) == 30 && page_value(&nb, 1) == 60 && page_value(&nb, 3) == 90);
    let v3 = page_value(&nb, 2);
    check!(v3 == 70);
    for i in 0..nb.npages() {
        let mut p: Option<Box<Page>> = None;
        check!(store_read_page(&nb, i, &mut p) == 0 && p.as_ref().is_some_and(|p| paper_is_document(p.paper)));
        if let Some(pg) = p {
            page_free(pg);
        }
    }
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    check!(count_prefix(&bg, "") >= 4 + 2 + 2); // 4 pictures, index.ini, pages.ini (and . ..)
    check!(index_entries(&nb) == 4);
    path = format!("{}/notebooks/{}/thumb.pgm", root(), id);
    check!(exists(&path));
    // The folder moved to import/done, its mark gone; nothing half-built is left.
    let done = format!("{}/import/done", root());
    check!(!exists(&src) && count_prefix(&done, "Score-") == 1);
    path = format!("{}/notebooks", root());
    check!(count_prefix(&path, ".import-") == 0);
    // A page added after the last (a document page) is blank: the notebook's paper.
    let after_last = nb.npages();
    check!(store_add_page(&mut nb, after_last, PAPER_BLANK) == 0);
    let mut p: Option<Box<Page>> = None;
    check!(store_load_page(&nb, 4, PW, PH, &mut p) == 0 && p.as_ref().is_some_and(|p| p.paper == PAPER_BLANK));
    if let Some(pg) = p.take() {
        page_free(pg);
    }
    // The raw zip carries the pictures.
    let mut b = Buf::default();
    {
        let mut z = zip_begin(&mut b);
        check!(export_notebook_zip(&nb, DPI, &mut z, "") == 0 && zip_end(&mut z) == 0);
    }
    let zp = format!("{}/score.zip", root());
    put_file(&zp, &b.data);
    buf_free(&mut b);
    let mut first: Option<Box<Page>> = None;
    store_read_page(&nb, 0, &mut first);
    let cmd = cformat!(
        "unzip -tq '%s' >/dev/null && unzip -l '%s' | grep -q 'Score/raw/bg/%06x.png' && \
         unzip -l '%s' | grep -q 'Score/raw/bg/index.ini' && unzip -l '%s' | grep -q 'Score/raw/bg/pages.ini' && \
         unzip -l '%s' | grep -q 'Score/page-004.png'",
        &zp,
        &zp,
        first.as_ref().map_or(0u32, |f| paper_image_id(f.paper)),
        &zp,
        &zp,
        &zp
    );
    if let Some(pg) = first {
        page_free(pg);
    }
    check!(system(&cmd) == 0);
}

fn import_same_picture_twice() {
    let files = ["a.png", "b.png"];
    let values: [u8; 2] = [120, 120];
    let src = import_folder_of("Twins", &files, &values, 2);
    let id = "20261020-120001-beef";
    mark(&src, id);
    check!(bg_import_folder(&src, DPI) == 0);
    let mut nb = NotebookInfo::default();
    check!(store_load_info(id, &mut nb) == 0 && nb.npages() == 2 && index_entries(&nb) == 1);
    let (mut a, mut b): (Option<Box<Page>>, Option<Box<Page>>) = (None, None);
    check!(
        store_read_page(&nb, 0, &mut a) == 0
            && store_read_page(&nb, 1, &mut b) == 0
            && matches!((&a, &b), (Some(a), Some(b)) if a.paper == b.paper)
    );
    if let Some(pg) = a {
        page_free(pg);
    }
    if let Some(pg) = b {
        page_free(pg);
    }
}

// A folder name longer than a notebook name holds (95 bytes) is cut before a character, never
// inside one.
fn import_long_name() {
    let name = "\u{e9}".repeat(50); // 50 e-acute: 100 bytes
    let files = ["p.png"];
    let values: [u8; 1] = [80];
    let src = import_folder_of(&name, &files, &values, 1);
    let id = "20261020-120003-cafe";
    mark(&src, id);
    check!(bg_import_folder(&src, DPI) == 0);
    let mut nb = NotebookInfo::default();
    check!(store_load_info(id, &mut nb) == 0 && nb.name.len() == 94 && nb.name.as_bytes() == &name.as_bytes()[..94]);
}

fn import_failure_leaves_nothing() {
    let files = ["page-1.png", "page-2.png", "page-3.png"];
    let values: [u8; 3] = [50, 1, 60]; // page 2 is damaged
    let src = import_folder_of("Broken", &files, &values, 3);
    let id = "20261020-120002-dead";
    mark(&src, id);
    let mut before = Vec::new();
    check!(store_list(&mut before) == 0);
    let nbefore = before.len();
    log_capture();
    check!(bg_import_folder(&src, DPI) != 0);
    // Refused from its headers, before any page was made ("N page(s), for WxH pages" starts that).
    let log = log_captured();
    check!(log.as_ref().is_some_and(|l| l.contains("page 2 of 3: not a PNG") && !l.contains("page(s), for")));
    let mut nb = NotebookInfo::default();
    let mut after = Vec::new();
    check!(store_load_info(id, &mut nb) != 0);
    check!(store_list(&mut after) == 0 && after.len() == nbefore);
    let mut path = format!("{}/notebooks", root());
    check!(count_prefix(&path, ".import-") == 0);
    for f in files {
        path = format!("{src}/{f}");
        check!(exists(&path));
    }
    // The folder is as it was (its mark gone), and the refusal is remembered (import_scan).
    path = format!("{src}/.grimoire-import");
    check!(!exists(&path));
    path = format!("{}/import/.attempts", root());
    let txt = slurp_text(&path);
    check!(txt.as_ref().is_some_and(|t| t.contains(" 99\n")));
    // Remove the damage: the next start imports it.
    path = format!("{src}/page-2.png");
    unlink(&path);
}

// Runs the imports import_scan queued, as app.c does: the results import_finished gives, in order.
fn finish_imports(results: &mut [i32], max: usize, pages: &mut [i32]) -> usize {
    let mut got = 0;
    let until = now_ms().wrapping_add(90000);
    let mut last = false;
    while !last {
        last = !bg_busy() || (now_ms().wrapping_sub(until) as i32) >= 0;
        let mut kind = -1;
        let r = bg_poll_kind(Some(&mut kind));
        if kind == BgKind::ImportFolder as i32 && got < max {
            results[got] = import_finished(r > 0, &mut pages[got]);
            got += 1;
        }
        if !last {
            usleep(5000);
        }
    }
    got
}

fn notebooks_named(name: &str) -> i32 {
    let mut all = Vec::new();
    if store_list(&mut all) != 0 {
        return -1;
    }
    all.iter().filter(|nb| nb.name == name).count() as i32
}

// A page whose header comes after a lot of metadata (an ICC profile, text) is still a page: the
// headers are checked from each file's first 64 KB, and from the whole file when that is not enough.
fn import_big_header() {
    let src = format!("{}/import/Annotated", root());
    let mut g = solid(150, 200, 1, 110);
    let mut path = format!("{src}/1.png");
    padded_png(&path, &g, 300usize << 10);
    path = format!("{src}/2.png");
    put_png(&path, &g);
    bgpicture_free(&mut g);
    let (mut res, mut pages) = ([0i32; 2], [0i32; 2]);
    check!(import_scan(DPI) == 1 && finish_imports(&mut res, 2, &mut pages) == 1 && res[0] == 1 && pages[0] == 2);
    check!(notebooks_named("Annotated") == 1 && !exists(&src));
}

// A folder whose page is no picture fails once (one toast), then waits for its pictures to change,
// however many starts there are; changed, it is imported.
fn import_refused_waits() {
    let files = ["page-1.png", "page-2.png"];
    let values: [u8; 2] = [40, 1]; // page 2 is text
    let src = import_folder_of("Waits", &files, &values, 2);
    let (mut res, mut pages) = ([0i32; 4], [0i32; 4]);
    check!(import_scan(DPI) == 1 && finish_imports(&mut res, 4, &mut pages) == 1 && res[0] == -1);
    let mut path = format!("{src}/.grimoire-import");
    check!(!exists(&path));
    for _start in 0..3 {
        check!(import_scan(DPI) == 0 && !bg_busy());
    }
    check!(notebooks_named("Waits") == 0);
    // The page replaced by a picture: taken at the next start.
    path = format!("{src}/page-2.png");
    let mut g = solid(150, 200, 1, 90);
    unlink(&path);
    put_png(&path, &g);
    bgpicture_free(&mut g);
    check!(import_scan(DPI) == 1 && finish_imports(&mut res, 4, &mut pages) == 1 && res[0] == 1 && pages[0] == 2);
    check!(notebooks_named("Waits") == 1 && !exists(&src));
    path = format!("{}/import/.attempts", root());
    let txt = slurp_text(&path);
    check!(txt.as_ref().is_none_or(|t| !t.contains(" 99\n"))); // (the record went with the folder)
}

// A failure that may pass (here: notebooks/ cannot be written) is tried at three starts, then the
// folder waits for its pictures to change, so a failure that comes back costs three tries, not one
// at every start: an import cut off each time (memory, a crash) is the same.
fn import_retries_three_times() {
    if geteuid() == 0 {
        return; // (root writes anywhere)
    }
    let files = ["a.png"];
    let values: [u8; 1] = [70];
    let src = import_folder_of("Thrice", &files, &values, 1);
    let nbdir = format!("{}/notebooks", root());
    check!(chmod(&nbdir, 0o555) == 0);
    let (mut res, mut pages) = ([0i32; 4], [0i32; 4]);
    let mut tried = 0;
    for _start in 0..5 {
        if import_scan(DPI) == 1 {
            tried += 1;
            check!(finish_imports(&mut res, 4, &mut pages) == 1 && res[0] == -1);
        }
    }
    check!(chmod(&nbdir, 0o755) == 0);
    check!(tried == 3 && notebooks_named("Thrice") == 0);
    // Touched (a file added), it is tried again.
    let path = format!("{src}/b.png");
    let mut g = solid(150, 200, 1, 80);
    put_png(&path, &g);
    bgpicture_free(&mut g);
    check!(import_scan(DPI) == 1 && finish_imports(&mut res, 4, &mut pages) == 1 && res[0] == 1 && pages[0] == 2);
}

// Cut off between the notebook's rename into place and its folder's move: the next start finishes
// the move (the folder's mark names the notebook made) and makes no second notebook.
fn import_made_before_moved() {
    let files = ["p-1.png", "p-2.png"];
    let values: [u8; 2] = [20, 30];
    let src = import_folder_of("Twice", &files, &values, 2);
    let id = "20261020-120005-f00d";
    mark(&src, id);
    check!(bg_import_folder(&src, DPI) == 0 && notebooks_named("Twice") == 1);
    // Put the folder back as the cut-off run left it.
    let done = format!("{}/import/done", root());
    let mut back = String::new();
    if let Ok(rd) = std::fs::read_dir(&done) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("Twice-") {
                back = format!("{done}/{name}");
            }
        }
    }
    check!(std::fs::rename(&back, &src).is_ok());
    mark(&src, id);
    let (mut res, mut pages) = ([0i32; 4], [0i32; 4]);
    check!(import_scan(DPI) == 1 && finish_imports(&mut res, 4, &mut pages) == 1 && res[0] == 1 && pages[0] == 2);
    check!(notebooks_named("Twice") == 1 && !exists(&src) && count_prefix(&done, "Twice-") == 1);
}

// A notebook an import had finished (its notebook.ini written, last) but whose rename into place
// was lost (a power cut after the folder's move was durable and before the rename was): the next
// start puts it in place, never removes it.
fn import_leftover_whole() {
    let files = ["only.png"];
    let values: [u8; 1] = [60];
    let src = import_folder_of("Kept", &files, &values, 1);
    let id = "20261020-120006-beef";
    let lost = "20261020-120007-cafe";
    mark(&src, id);
    check!(bg_import_folder(&src, DPI) == 0);
    let from = format!("{}/notebooks/{}", root(), id);
    let to = format!("{}/notebooks/.import-{}", root(), lost);
    check!(std::fs::rename(&from, &to).is_ok() && notebooks_named("Kept") == 0);
    check!(import_scan(DPI) == 0);
    let mut nb = NotebookInfo::default();
    check!(!exists(&to) && store_load_info(lost, &mut nb) == 0 && nb.npages() == 1 && nb.name == "Kept");
    check!(page_value(&nb, 0) == 60);
}

fn import_at_startup() {
    // What a cut-off import left: removed. A folder without pictures and import/done: left alone.
    let stale = format!("{}/notebooks/.import-20261019-090000-0000", root());
    let mut path = format!("{stale}/bg/00abcd.png");
    put_file(&path, b"half");
    path = format!("{stale}/p1.qpg");
    put_file(&path, b"half");
    path = format!("{}/import/Empty/readme.txt", root());
    put_file(&path, b"nothing");
    // "Broken" (from the test above) is whole now: it is imported again, with "Lieder".
    let files = ["01.jpg.png", "02.png"];
    let values: [u8; 2] = [10, 20];
    import_folder_of("Lieder", &files, &values, 2);
    let queued = import_scan(DPI);
    check!(queued == 2);
    check!(!exists(&stale));
    let (mut got, mut pages, mut total) = (0, 0, 0);
    let until = now_ms().wrapping_add(90000);
    while got < 2 && (now_ms().wrapping_sub(until) as i32) < 0 {
        let mut kind = -1;
        let r = bg_poll_kind(Some(&mut kind));
        if kind == BgKind::ImportFolder as i32 {
            let f = import_finished(r > 0, &mut pages);
            check!(f == 1);
            if f == 1 {
                total += pages;
            }
            got += 1;
        }
        usleep(5000);
    }
    check!(got == 2 && total == 4); // Broken: 2 pages, Lieder: 2
    path = format!("{}/import/Empty/readme.txt", root());
    check!(exists(&path));
    path = format!("{}/import", root());
    check!(count_prefix(&path, "Lieder") == 0 && count_prefix(&path, "Broken") == 0);
    // Nothing left to import: a second start queues nothing.
    check!(import_scan(DPI) == 0 && !bg_busy());
}

// A lock that cannot be taken for another reason than an import holding it says why (it is not
// "an import from an earlier run is still going"); the folders wait for the next start either way.
fn import_lock_errors() {
    let files = ["x.png"];
    let values: [u8; 1] = [90];
    import_folder_of("Locked", &files, &values, 1);
    let lock = format!("{}/import/.lock", root());
    unlink(&lock);
    check!(mkdir(&lock) == 0); // (no file can be opened there)
    log_capture();
    check!(import_scan(DPI) == 0 && !bg_busy());
    let log = log_captured();
    check!(log.as_ref().is_some_and(|l| l.contains("lock cannot be taken (") && !l.contains("still going")));
    check!(std::fs::remove_dir(&lock).is_ok());
    let (mut res, mut pages) = ([0i32; 2], [0i32; 2]);
    check!(import_scan(DPI) == 1 && finish_imports(&mut res, 2, &mut pages) == 1 && res[0] == 1 && pages[0] == 1);
}

// A folder or template whose path would not reach a background job whole (a job holds 600 bytes:
// a long name in another script, under vfat's 255 characters) is left out, never cut and then
// tried at every start. Here a deep root makes the paths long. (Last: it moves the store.)
fn long_paths_left_out() {
    let mut deep = String::new();
    csnprintf!(&mut deep, 512, "%s/%s/", &root(), &"d".repeat(200));
    deep.push_str(&"e".repeat(150));
    check!(deep.len() < 500 && store_init(&deep) == 0);
    templates_init();
    let name = "n".repeat(230);
    let mut g = solid(150, 200, 1, 100);
    let mut path = format!("{deep}/import/{name}/p.png");
    put_png(&path, &g);
    path = format!("{deep}/templates/{name}.png");
    put_png(&path, &g);
    let shortp = format!("{deep}/templates/short.png");
    put_png(&shortp, &g);
    check!(import_scan(DPI) == 0 && !bg_busy());
    path = format!("{deep}/import/{name}/.grimoire-import");
    check!(!exists(&path));
    let mut t: Vec<Template> = Vec::new();
    check!(template_scan(&mut t, DPI) == 0 && t.len() == 1 && t[0].name == "short");
    template_list_free(&mut t);
    drain_jobs(None);
    let mut nb = NotebookInfo::default();
    let mut k = PaperKind::default();
    let mut why: &'static str = "";
    check!(store_create("Deep", "", PAPER_BLANK, &mut nb) == 0);
    path = format!("{deep}/templates/{name}.png");
    check!(template_install(&nb, &path, &mut k, &mut why) != 0 && why.contains("long"));
    bgpicture_free(&mut g);
}

// ---- photos ----

// A JPEG's start with an APP1 Exif block whose IFD0 holds Orientation = v (le: little-endian TIFF).
fn exif_jpeg(out: &mut [u8], v: i32, le: bool, more_tags: bool) -> usize {
    let mut n = 0usize;
    for b in [0xFFu8, 0xD8, 0xFF, 0xE0, 0, 4, b'J', b'F'] {
        // an APP0 first, as many cameras write
        out[n] = b;
        n += 1;
    }
    let app1 = n;
    out[n] = 0xFF;
    out[n + 1] = 0xE1;
    n += 2;
    n += 2; // (its length, below)
    out[n..n + 6].copy_from_slice(b"Exif\0\0");
    n += 6;
    out[n] = if le { b'I' } else { b'M' };
    out[n + 1] = out[n];
    n += 2;
    macro_rules! p16 {
        ($x:expr) => {{
            let x = $x as u32;
            if le {
                out[n] = x as u8;
                out[n + 1] = (x >> 8) as u8;
            } else {
                out[n] = (x >> 8) as u8;
                out[n + 1] = x as u8;
            }
            n += 2;
        }};
    }
    macro_rules! p32 {
        ($x:expr) => {{
            let x = $x as u32;
            if le {
                p16!(x & 0xFFFF);
                p16!(x >> 16);
            } else {
                p16!(x >> 16);
                p16!(x & 0xFFFF);
            }
        }};
    }
    p16!(42);
    p32!(8);
    let count = if more_tags { 3 } else { 1 };
    p16!(count);
    if more_tags {
        // ImageWidth and ImageLength first, as a camera writes them
        p16!(0x0100);
        p16!(4);
        p32!(1);
        p32!(4000);
        p16!(0x0101);
        p16!(4);
        p32!(1);
        p32!(3000);
    }
    p16!(0x0112);
    p16!(3);
    p32!(1);
    p16!(v);
    p16!(0);
    p32!(0);
    let len = n - app1 - 2;
    out[app1 + 2] = (len >> 8) as u8;
    out[app1 + 3] = len as u8;
    for b in [0xFFu8, 0xDA, 0, 2] {
        // then the frame (no more needed: the parser stops at Exif)
        out[n] = b;
        n += 1;
    }
    n
}

fn exif_orientation() {
    let mut j = [0u8; 256];
    for le in [false, true] {
        for more in [false, true] {
            for v in 1..=8 {
                let n = exif_jpeg(&mut j, v, le, more);
                check!(bgimage_exif_orientation(&j[..n]) == v);
            }
        }
    }
    // Values out of range, no Exif, damaged blocks, other files: 1, nothing turned.
    let mut n = exif_jpeg(&mut j, 9, false, false);
    check!(bgimage_exif_orientation(&j[..n]) == 1);
    n = exif_jpeg(&mut j, 0, true, true);
    check!(bgimage_exif_orientation(&j[..n]) == 1);
    n = exif_jpeg(&mut j, 6, false, true);
    for cut in 0..n - 4 {
        check!(bgimage_exif_orientation(&j[..cut]) == 1 || cut >= n - 4);
    }
    let mut bad = [0u8; 256];
    bad[..n].copy_from_slice(&j[..n]);
    bad[13] = b'X'; // "Exif" no more
    check!(bgimage_exif_orientation(&bad[..n]) == 1);
    bad[..n].copy_from_slice(&j[..n]);
    bad[20] = 0x7F; // IFD0 offset past the block
    check!(bgimage_exif_orientation(&bad[..n]) == 1);
    bad[..n].copy_from_slice(&j[..n]);
    bad[9] = 0xFF; // APP1's length past the file
    check!(bgimage_exif_orientation(&bad[..n]) == 1);
    const PNG: [u8; 12] = [0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13];
    check!(bgimage_exif_orientation(&PNG) == 1);
    check!(bgimage_exif_orientation(&j[..0]) == 1);
    // The fixture a phone would write: stored sideways, orientation 6.
    let d = slurp(&fixture("photos/IMG_0001.jpg"));
    check!(d.as_ref().is_some_and(|d| bgimage_exif_orientation(d) == 6));
}

// Each EXIF orientation, on a 3x2 picture a b c / d e f (in both gray and RGB): what shows upright.
fn orientations() {
    let want = ["", "abc def", "cba fed", "fed cba", "def abc", "ad be cf", "da eb fc", "fc eb da", "cf be ad"];
    let mut ch = 1;
    while ch <= 3 {
        for o in 1..=8usize {
            let mut p = picture(3, 2, ch);
            for i in 0..6usize {
                p.px[i * ch as usize..(i + 1) * ch as usize].fill(b'a' + i as u8);
            }
            check!(bgimage_orient(&mut p, o as i32) == 0);
            let mut got = String::new();
            for y in 0..p.h {
                if y != 0 {
                    got.push(' ');
                }
                for x in 0..p.w {
                    got.push(p.px[(y as usize * p.w as usize + x as usize) * ch as usize] as char);
                }
            }
            if got != want[o] {
                eprintln!("  orientation {o}, {ch} channel(s): {got}, wanted {}", want[o]);
            }
            check!(got == want[o]);
            check!(if o >= 5 { p.w == 2 && p.h == 3 } else { p.w == 3 && p.h == 2 });
            bgpicture_free(&mut p);
        }
        ch += 2;
    }
}

// A photo from grimoire/photos: a page's own picture (a document page), upright (the fixture's black
// block at its top left, its gray bar at the bottom), of a key of its own kind: the same file as a
// template is another picture, as it is stored (sideways).
fn photo_upright_and_its_own() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Photos", "", PAPER_MEDIUM, &mut nb) == 0);
    let path = format!("{}/photos/IMG_0001.jpg", root());
    let d = slurp(&fixture("photos/IMG_0001.jpg"));
    check!(d.is_some());
    if let Some(d) = &d {
        put_file(&path, d);
    }
    check!(photo_path(&path));
    let tpl = format!("{}/templates/IMG_0001.jpg", root());
    check!(!photo_path(&tpl));
    let (mut k, mut kt) = (PAPER_BLANK, PAPER_BLANK);
    let mut why: &'static str = "";
    check!(photo_install(&nb, &path, &mut k, &mut why) == 0 && paper_is_image(k) && paper_is_document(k));
    if let Some(d) = slurp(&path) {
        put_file(&tpl, &d);
    }
    check!(template_install(&nb, &tpl, &mut kt, &mut why) == 0 && paper_is_image(kt) && !paper_is_document(kt));
    check!(paper_image_id(kt) != paper_image_id(k));
    drain_jobs(None);
    let (mut dir, mut file) = (String::new(), String::new());
    bgimage_dir(nb.id.as_str(), &mut dir, 700);
    bgimage_file(&dir, paper_image_id(k), &mut file, 800);
    let d = slurp(&file);
    let mut made = BgPicture::default();
    check!(d.as_ref().is_some_and(|d| bgimage_decode(d, &mut made, &mut why) == 0) && made.w == PW && made.h == PH);
    // Fitted upright, it fills the page (3:4 both): the block covers the top left 30% of its width and
    // 22% of its height, the bar its bottom 10%.
    let atp = |pic: &BgPicture, x: i32, y: i32| pic.px[(y as usize * pic.w as usize + x as usize) * pic.ch as usize];
    if !made.px.is_empty() {
        let tl = atp(&made, PW / 10, PH / 10);
        let tr = atp(&made, PW - PW / 10, PH / 10);
        let bottom = atp(&made, PW / 2, PH - PH / 20);
        if !(tl < 40 && tr > 215 && bottom > 100 && bottom < 156) {
            eprintln!("  photo: top left {tl}, top right {tr}, bottom {bottom} (ch {})", made.ch);
        }
        check!(tl < 40 && tr > 215 && bottom > 100 && bottom < 156);
    }
    bgpicture_free(&mut made);
    // As a template the same file is as stored, sideways: fitted into a band across the page's middle
    // (400x300 at a scale of PW / 400), white above it, the block at the band's bottom left.
    bgimage_file(&dir, paper_image_id(kt), &mut file, 800);
    let d = slurp(&file);
    check!(d.as_ref().is_some_and(|d| bgimage_decode(d, &mut made, &mut why) == 0));
    if !made.px.is_empty() {
        let band = PW * 300 / 400;
        let top = (PH - band) / 2;
        check!(atp(&made, PW / 10, PH / 10) > 215 && atp(&made, PW / 20, top + band - band / 10) < 40);
    }
    bgpicture_free(&mut made);
    // Its preview is upright too, and photo_paper finds it again from the preview's key.
    let mut t: Vec<Template> = Vec::new();
    check!(photos_scan(&mut t, DPI) == 0 && t.len() == 1);
    drain_jobs(None);
    template_list_free(&mut t);
    check!(photos_scan(&mut t, DPI) == 0 && t.len() == 1 && t[0].ready);
    let nt = t.len();
    let pv = if nt != 0 { template_thumb(&mut t[0]) } else { None };
    check!(pv.is_some_and(|pv| {
        pv.w == PW / 6 && pv.px[16 * pv.stride as usize + 16] < 40 && pv.px[16 * pv.stride as usize + (pv.w - 16) as usize] > 215
    }));
    let mut ix = BgIndex::default();
    let mut again = PAPER_BLANK;
    check!(bgindex_load(&dir, &mut ix) == 0 && nt != 0 && photo_paper(&t[0], &nb, &ix, &mut again) == 0 && again == k);
    bgindex_free(&mut ix);
    template_list_free(&mut t);
    unlink(&path);
    unlink(&tpl);
}

// Everything, in the C's order (the tests share the store and the folders the ones before them made).
#[test]
fn test_templates() {
    let top = format!("{}/templates-{}", tl_tmp(), std::process::id());
    let _ = ROOT.set(top.clone());
    let _ = std::fs::remove_dir_all(&top);
    check!(store_init(&top) == 0);
    store_set_device(DPI, PW, PH);
    run!(image_kinds);
    run!(image_paper_is_pure);
    run!(missing_picture_draws_white);
    run!(decoding);
    run!(oversized_refused);
    run!(decode_budget);
    run!(ui_picture_limits);
    run!(normalization);
    run!(templates_same_file_same_id);
    run!(exif_orientation);
    run!(orientations);
    run!(photo_upright_and_its_own);
    run!(source_key_streamed);
    run!(templates_collision_steps);
    run!(same_id_two_notebooks);
    run!(template_previews);
    run!(big_template_uses_its_preview_key);
    run!(document_role_not_inherited);
    run!(sidecar_restores_beta_resave);
    run!(beta_reads_image_pages);
    run!(erasing_over_a_picture);
    run!(listed_pages_restored_at_open);
    run!(notebook_on_a_template);
    run!(card_drawn_again_with_its_picture);
    run!(card_written_meanwhile);
    run!(card_replaced_only_if_same);
    run!(new_notebook_template_refused);
    run!(unreadable_index_kept);
    run!(unreadable_template_tried_later);
    run!(natural_order);
    run!(import_makes_a_notebook);
    run!(import_same_picture_twice);
    run!(import_long_name);
    run!(import_failure_leaves_nothing);
    run!(import_at_startup);
    run!(import_big_header);
    run!(import_refused_waits);
    run!(import_retries_three_times);
    run!(import_made_before_moved);
    run!(import_leftover_whole);
    run!(import_lock_errors);
    run!(long_paths_left_out);
    done();
    let _ = std::fs::remove_dir_all(&top);
}
