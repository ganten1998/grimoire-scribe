// test_picwork.rs: pictures in the sandbox (2.1.0-beta.2, DESIGN.md 3.7 and 9 "L7"). Every picture goes through
// a real worker, a fresh exec of the headless runner (GRIMOIRE_WORK_EXE), as in the app. Hostile pictures are
// generated here. The Linux sandbox steps need root and run in the CI job (test_sandbox_linux.rs); the
// fallback, which makes the process decode pictures itself for good, has its own binary (test_picwork_fallback.rs).
#![cfg(feature = "sim")]
mod testlib;
use grimoire::picwork::*;
use grimoire::prelude::*;
use testlib::*;

const PW: i32 = 600;
const PH: i32 = 800;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

const HOOKS: [&str; 6] = [
    "GRIMOIRE_PIC_TEST_CRASH",
    "GRIMOIRE_PIC_TEST_HANG",
    "GRIMOIRE_PIC_TEST_TRAP",
    "GRIMOIRE_PIC_WATCHDOG_MS",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
}

fn gradient(w: i32, h: i32, ch: i32) -> BgPicture {
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            for c in 0..ch {
                px.push(((x * 3 + y * 5 + c * 40) % 251) as u8);
            }
        }
    }
    BgPicture { px, w, h, ch }
}

fn png_bytes(p: &BgPicture, dir: &str) -> Vec<u8> {
    let path = format!("{dir}/gen.png");
    assert_eq!(bgimage_write_png(&path, p), 0);
    std::fs::read(&path).unwrap()
}

fn png_header_of(w: u32, h: u32, depth: u8, color: u8) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    out.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    out.extend_from_slice(&w.to_be_bytes());
    out.extend_from_slice(&h.to_be_bytes());
    out.extend_from_slice(&[depth, color, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&[0, 0, 0, 0, b'I', b'D', b'A', b'T', 0, 0, 0, 0]);
    out
}

struct Bits {
    d: Vec<u8>,
    acc: u32,
    bits: i32,
}
impl Bits {
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
    fn put_code(&mut self, code: u32, n: i32) {
        for i in (0..n).rev() {
            self.put_bits((code >> i) & 1, 1);
        }
    }
}

// A 1x1 gray PNG whose IDAT inflates to `bytes` zeros: a zlib bomb.
fn png_bomb(bytes: usize) -> Vec<u8> {
    let mut hdr = png_header_of(1, 1, 8, 0);
    hdr.truncate(hdr.len() - 12);
    let mut b = Bits { d: vec![0x78, 0x01], acc: 0, bits: 0 };
    b.put_bits(1, 1);
    b.put_bits(1, 2);
    b.put_code(0x30, 8);
    for _ in 0..bytes / 258 {
        b.put_code(0xC5, 8);
        b.put_code(0, 5);
    }
    b.put_code(0, 7);
    if b.bits != 0 {
        let rest = 8 - b.bits;
        b.put_bits(0, rest);
    }
    let bn = b.d.len() + 4;
    let mut png = hdr;
    png.extend_from_slice(&(bn as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&b.d);
    png.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
    png.extend_from_slice(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]);
    png
}

fn tmp(name: &str) -> String {
    let d = format!("{}/picwork-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

// The worker's answer is the writer's own decode of 2.0, byte for byte: same fit, same channels, same size.
fn worker_matches_the_writers_decode() {
    clean_env();
    let dir = tmp("same");
    let mut pool = PicPool::new();
    for ch in [1, 3] {
        let src = gradient(321, 457, ch);
        let bytes = png_bytes(&src, &dir);
        let mut local = BgPicture::default();
        let mut why: &'static str = "";
        check!(bgimage_decode(&bytes, &mut local, &mut why) == 0);
        let mut want = BgPicture::default();
        check!(bgimage_normalize(&local, PW, PH, &mut want) == 0);
        match pool.decode(&bytes, false, PW, PH) {
            PicOut::Pic { src_w, src_h, fit } => {
                check!(src_w == 321 && src_h == 457);
                check!(fit.w == PW && fit.h == PH && fit.ch == ch && fit.px == want.px);
            }
            o => {
                eprintln!("  not a picture: {o:?}");
                check!(false);
            }
        }
        match pool.check(&bytes[..bytes.len().min(65536)], bytes.len()) {
            PicOut::Info { w, h } => check!(w == 321 && h == 457),
            _ => check!(false),
        }
    }
    check!(!in_fallback());
}

// A photo is turned upright in the worker (the EXIF orientation), as the writer did.
fn photo_is_turned_in_the_worker() {
    clean_env();
    let mut pool = PicPool::new();
    // a JPEG with an orientation is too much to generate here: orientation 1 (as is) and a PNG (none)
    let dir = tmp("photo");
    let bytes = png_bytes(&gradient(200, 100, 1), &dir);
    let a = pool.decode(&bytes, true, PW, PH);
    let b = pool.decode(&bytes, false, PW, PH);
    match (a, b) {
        (PicOut::Pic { fit: a, .. }, PicOut::Pic { fit: b, .. }) => check!(a.px == b.px),
        _ => check!(false),
    }
}

// The hostile corpus: each is refused with a reason, none ends the worker, and the next good picture still
// decodes through the same pool.
fn hostile_pictures_are_refused() {
    clean_env();
    let dir = tmp("hostile");
    let good = png_bytes(&gradient(64, 64, 1), &dir);
    let mut pool = PicPool::new();
    let mut corpus: Vec<(&str, Vec<u8>, &str)> = vec![
        ("garbage", b"this is no picture at all".to_vec(), "PNG, JPEG or PNM"),
        ("truncated", good[..good.len() / 2].to_vec(), "damaged"),
        ("header only", good[..33].to_vec(), ""),
        ("huge a side", png_header_of(100_000, 10, 8, 0), "8192"),
        ("huge area", png_header_of(8000, 6000, 8, 0), "40 megapixels"),
        ("16-bit rgba", png_header_of(7900, 5000, 16, 6), "memory"),
        ("bomb", png_bomb(64 << 20), "damaged"),
    ];
    let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0];
    jpeg.extend_from_slice(&[0xFF; 40]);
    corpus.push(("jpeg head", jpeg, ""));
    for (name, bytes, want) in &corpus {
        match pool.decode(bytes, false, PW, PH) {
            PicOut::Bad(why) => check!(want.is_empty() || why.contains(want)),
            o => {
                eprintln!("  {name}: {o:?}");
                check!(false);
            }
        }
        match pool.check(&bytes[..bytes.len().min(65536)], bytes.len()) {
            PicOut::Pic { .. } => check!(false),
            PicOut::Dead(_) => check!(false),
            _ => {}
        }
        // (the worker is still there, and the next good picture decodes)
        check!(matches!(pool.decode(&good, false, PW, PH), PicOut::Pic { .. }));
    }
}

// A worker that dies on a picture (crash, trap) or stalls on it fails that picture alone: Dead, and the pool's
// next picture goes to a fresh worker.
fn a_worker_that_dies_fails_one_picture() {
    clean_env();
    let dir = tmp("dead");
    let good = png_bytes(&gradient(64, 64, 1), &dir);
    // (the trap hook needs the seccomp filter: the Linux job)
    for hook in ["GRIMOIRE_PIC_TEST_CRASH", "GRIMOIRE_PIC_TEST_HANG"] {
        clean_env();
        let mut pool = PicPool::new();
        check!(matches!(pool.decode(&good, false, PW, PH), PicOut::Pic { .. })); // (worker 1 serves one)
        std::env::set_var(hook, "2");
        if hook == "GRIMOIRE_PIC_TEST_HANG" {
            std::env::set_var("GRIMOIRE_PIC_WATCHDOG_MS", "1500");
        }
        // a fresh pool takes the hook: its worker's second picture dies
        let mut pool = PicPool::new();
        check!(matches!(pool.decode(&good, false, PW, PH), PicOut::Pic { .. }));
        check!(matches!(pool.decode(&good, false, PW, PH), PicOut::Dead(_)));
        std::env::remove_var(hook);
        check!(matches!(pool.decode(&good, false, PW, PH), PicOut::Pic { .. })); // (a fresh worker)
        check!(!in_fallback());
    }
}

// One worker serves a slice of pictures, then a fresh exec: 100 pictures work, whatever the slice's size.
fn many_pictures_cross_slices() {
    clean_env();
    let dir = tmp("slices");
    let good = png_bytes(&gradient(40, 40, 1), &dir);
    let mut pool = PicPool::new();
    let n = 70;
    let mut ok = 0;
    for _ in 0..n {
        if matches!(pool.decode(&good, false, 200, 300), PicOut::Pic { .. }) {
            ok += 1;
        }
    }
    check!(ok == n);
}

// Through the background jobs: a template whose picture kills its worker is marked bad (and the log says
// which), the next scan does not try it again.
fn a_template_that_kills_its_worker_is_marked_bad() {
    clean_env();
    let top = tmp("job");
    check!(store_init(&top) == 0);
    store_set_device(300.0, PW, PH);
    templates_init();
    let path = format!("{top}/templates/Killer.png");
    let _ = std::fs::write(&path, png_bytes(&gradient(100, 120, 1), &top));
    std::env::set_var("GRIMOIRE_PIC_TEST_CRASH", "1");
    let mut t: Vec<Template> = Vec::new();
    check!(template_scan(&mut t, 300.0) == 0);
    template_list_free(&mut t);
    let until = now_ms() + 60_000;
    while bg_busy() && now_ms() < until {
        let _ = bg_poll_kind(None);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let _ = bg_poll_kind(None);
    std::env::remove_var("GRIMOIRE_PIC_TEST_CRASH");
    check!(template_scan(&mut t, 300.0) == 0);
    let found = t.iter().find(|x| x.name == "Killer").map(|x| (x.bad, x.ready));
    check!(found == Some((true, false)));
    template_list_free(&mut t);
    check!(!bg_busy());
}

#[test]
fn test_picwork() {
    run!(worker_matches_the_writers_decode);
    run!(photo_is_turned_in_the_worker);
    run!(hostile_pictures_are_refused);
    run!(a_worker_that_dies_fails_one_picture);
    run!(many_pictures_cross_slices);
    run!(a_template_that_kills_its_worker_is_marked_bad);
    done();
}
