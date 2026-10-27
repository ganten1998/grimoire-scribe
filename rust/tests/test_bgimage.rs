// test_bgimage.rs: bgimage.rs against the C build. bgimage.c has no test of its own (test_templates.c
// and test_annotate.c, which belong to later waves, use it), so this port of it was checked against
// the C itself: tests/fixtures/bgimage_probe.c is a C program (it links ../src/bgimage.c, canvas.c,
// util.c and stb_impl.c, built with clang -O1 as the Makefile's test flags have it) that runs every
// function of bgimage.h over the pictures in tests/fixtures/bgimage (PNGs of every depth and colour
// type, interlaced and not, with and without transparency, PNM, progressive and baseline JPEGs, Exif
// blocks of both byte orders, headers that claim more than a picture may be, a zlib bomb, damaged
// files) and prints one line a result, as hashes; fixtures/bgimage_probe_c.txt is what it printed.
// This test makes the same calls on the same data and compares section by section, line by line:
// the content keys, the area filter (bgimage_normalize), orient, luma, the Exif parser (and a fuzz
// of 20000 damaged blocks), the decode cost and the memory a decode held, check and decode with their
// reasons, bg/index.ini (load, save, claim), the whole-file hash, bgimage_read's errno, and drawing
// (the cache, its least recently used slot, the remembered failures, the generation, the region that
// agrees with the whole page). To record the fixture again see bgimage_probe.c.
//
// The drawing section needs a notebook's folder (store_notebook_dir), so it waits for store.rs.
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]
mod testlib;
use grimoire::prelude::*;
use std::fmt::Write as _;
use testlib::*;

const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/bgimage");

macro_rules! w {
    ($out:expr, $($a:tt)*) => { write!($out, $($a)*).unwrap() };
}

fn root() -> String {
    format!("{}/bgimage-{}", tl_tmp(), std::process::id())
}

struct Gen(u32);
impl Gen {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0 >> 8
    }
}

fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn mkpic(w: i32, h: i32, ch: i32, seed: u32) -> BgPicture {
    let mut g = Gen(seed);
    let n = (w * h * ch) as usize;
    let px: Vec<u8> = (0..n).map(|_| g.next() as u8).collect();
    BgPicture { px, w, h, ch }
}

fn dump_pic(out: &mut String, tag: &str, p: &BgPicture) {
    let n = (p.w * p.h * p.ch) as usize;
    w!(out, "{} {}x{} ch{} fnv {:016x}\n", tag, p.w, p.h, p.ch, fnv(&p.px[..n]));
}

fn sec_keys(out: &mut String) {
    for &n in &[0usize, 1, 7, 8, 9, 15, 16, 17, 63, 64, 65, 1000, 65539] {
        let mut g = Gen(1000 + n as u32);
        let d: Vec<u8> = (0..n).map(|_| g.next() as u8).collect();
        let c = bgimage_content_hash(&d);
        w!(
            out,
            "hash {} {:016x} src {:016x} tpl {:016x} photo {:016x}\n",
            n,
            c,
            bgimage_source_key(&d, 1860, 2480),
            bgimage_template_key(c, n, 100, 200),
            bgimage_photo_key(c, n, 100, 200)
        );
    }
    let dims = [(5, 3, 3), (4, 4, 1), (1, 1, 3), (17, 9, 1)];
    for (i, d) in dims.iter().enumerate() {
        let pic = mkpic(d.0, d.1, d.2, 77 + i as u32);
        w!(out, "key {}x{} ch{} {:016x}\n", pic.w, pic.h, pic.ch, bgimage_key(&pic));
    }
}

fn sec_normalize(out: &mut String) {
    let cs: [[i32; 5]; 11] = [
        [7, 5, 1, 20, 20],
        [100, 60, 3, 31, 47],
        [64, 64, 3, 64, 64],
        [3, 200, 1, 100, 100],
        [500, 333, 3, 120, 160],
        [1, 1, 3, 9, 9],
        [300, 300, 1, 50, 10],
        [20, 10, 3, 200, 100],
        [17, 31, 1, 17, 31],
        [1000, 3, 1, 33, 40],
        [123, 456, 3, 61, 99],
    ];
    for (i, c) in cs.iter().enumerate() {
        let s = mkpic(c[0], c[1], c[2], 500 + i as u32);
        let mut o = BgPicture::default();
        let rc = bgimage_normalize(&s, c[3], c[4], &mut o);
        w!(out, "normalize {}\n", rc);
        dump_pic(out, "  out", &o);
        let b = BgPicture::default();
        let r1 = bgimage_normalize(&b, 10, 10, &mut o);
        let r2 = bgimage_normalize(&s, 0, 10, &mut o);
        let r3 = bgimage_normalize(&s, 10, -1, &mut o);
        w!(out, "normalize bad {} {} {}\n", r1, r2, r3);
        let mut lum = Canvas::default();
        if bgimage_luma(&s, &mut lum) == 0 {
            w!(
                out,
                "  luma {}x{} stride {} fnv {:016x}\n",
                lum.w,
                lum.h,
                lum.stride,
                fnv(&lum.px[..(lum.w * lum.h) as usize])
            );
        }
    }
}

fn sec_orient(out: &mut String) {
    let dims = [(5, 3, 3), (4, 7, 1), (1, 6, 3)];
    for (i, d) in dims.iter().enumerate() {
        for o in 0..=9 {
            let mut pic = mkpic(d.0, d.1, d.2, 900 + i as u32);
            let rc = bgimage_orient(&mut pic, o);
            dump_pic(out, &format!("orient {} rc {}", o, rc), &pic);
        }
    }
    let mut e = BgPicture { px: Vec::new(), w: 3, h: 3, ch: 3 };
    w!(out, "orient empty {}\n", bgimage_orient(&mut e, 6));
}

fn slurp(name: &str) -> Option<Vec<u8>> {
    let path = format!("{}/{}", FIX, name);
    bgimage_read(&path, 1 << 30)
}

fn sec_exif(out: &mut String) {
    let names = [
        "exif_le_1.jpg",
        "exif_le_2.jpg",
        "exif_le_3.jpg",
        "exif_le_4.jpg",
        "exif_le_5.jpg",
        "exif_le_6.jpg",
        "exif_le_7.jpg",
        "exif_le_8.jpg",
        "exif_be_1.jpg",
        "exif_be_2.jpg",
        "exif_be_3.jpg",
        "exif_be_4.jpg",
        "exif_be_5.jpg",
        "exif_be_6.jpg",
        "exif_be_7.jpg",
        "exif_be_8.jpg",
        "exif_0.jpg",
        "exif_9.jpg",
        "exif_pad.jpg",
        "base_420.jpg",
        "gray8.png",
        "tiny.bin",
        "empty.bin",
        "junk.bin",
    ];
    for n in names {
        let d = slurp(n);
        w!(out, "exif {} {}\n", n, d.as_ref().map_or(-99, |d| bgimage_exif_orientation(d)));
    }
    let d = slurp("exif_le_6.jpg").unwrap();
    let n = d.len();
    let mut m = vec![0u8; n];
    let mut acc: u64 = 0;
    let mut counts = [0i32; 10];
    let mut g = Gen(4242);
    for _ in 0..20000 {
        m.copy_from_slice(&d);
        let mut k = 1 + (g.next() % 3) as i32;
        while k > 0 {
            k -= 1;
            let v = g.next() as u8; // (clang evaluates the right side of the C assignment first)
            let at = (g.next() % 80) as usize;
            m[at] = v;
        }
        let cut = if g.next().is_multiple_of(4) { (g.next() as usize) % n } else { n };
        let o = bgimage_exif_orientation(&m[..cut]);
        counts[o as usize] += 1;
        acc = acc.wrapping_mul(31).wrapping_add(o as u64);
    }
    w!(out, "exif fuzz {:016x}", acc);
    for c in counts {
        w!(out, " {}", c);
    }
    w!(out, "\n");
}

fn sec_decode(out: &mut String) {
    let names = [
        "gray8.png",
        "rgb8.png",
        "rgba8.png",
        "ga8.png",
        "grayrgb.png",
        "grayrgb1.png",
        "rgb16.png",
        "gray16.png",
        "rgba16.png",
        "pal4.png",
        "pal8trns.png",
        "gray1.png",
        "interlaced_bad.png",
        "adam7.png",
        "cgbi.png",
        "fake_9000.png",
        "fake_7000sq.png",
        "fake_5000x4000.png",
        "fake_rgb16_6000x3000.png",
        "fake_rgb16_3000x3000.png",
        "fake_pal_6000x5000.png",
        "fake_inter_4000x4000.png",
        "fake_rgba_6000x6000.png",
        "fake_6000.png",
        "fake_6001.png",
        "fake_1bit_8000.png",
        "bomb.png",
        "short.png",
        "trunc.png",
        "junk.bin",
        "tiny.bin",
        "empty.bin",
        "p5.pgm",
        "p6.ppm",
        "p6_16.ppm",
        "p5_16.pgm",
        "p6_short.ppm",
        "p6_100.ppm",
        "base_420.jpg",
        "base_444.jpg",
        "base_422.jpg",
        "base_gray.jpg",
        "prog_420.jpg",
        "prog_444.jpg",
        "prog_gray.jpg",
        "exif_le_6.jpg",
        "exif_pad.jpg",
    ];
    for name in names {
        let Some(d) = slurp(name) else {
            w!(out, "{}: unreadable\n", name);
            continue;
        };
        let n = d.len();
        let (mut w, mut h) = (-1, -1);
        let mut why = "";
        let mut rc = bgimage_check(&d, n, &mut w, &mut h, &mut why);
        w!(out, "{}: check {} {} {} [{}]\n", name, rc, w, h, if rc != 0 { why } else { "" });
        w!(out, "  cost {}\n", bgimage_decode_cost(&d, n));
        if n > 20 {
            w = -1;
            h = -1;
            why = "";
            rc = bgimage_check(&d[..20], n, &mut w, &mut h, &mut why);
            w!(out, "  head20 {} {} {} [{}]\n", rc, w, h, if rc != 0 { why } else { "" });
        }
        w = -1;
        h = -1;
        why = "";
        rc = bgimage_drawable(&d, &mut w, &mut h, &mut why);
        w!(out, "  drawable {} {} {} [{}]\n", rc, w, h, if rc != 0 { why } else { "" });
        if n < 100000 || name.starts_with("fake_6000") || name == "fake_5000x4000.png" {
            let mut pic = BgPicture::default();
            why = "";
            rc = bgimage_decode(&d, &mut pic, &mut why);
            w!(out, "  decode {} [{}]", rc, if rc != 0 { why } else { "" });
            if rc == 0 {
                w!(
                    out,
                    " {}x{} ch{} fnv {:016x}",
                    pic.w,
                    pic.h,
                    pic.ch,
                    fnv(&pic.px[..(pic.w * pic.h * pic.ch) as usize])
                );
            }
            w!(out, " peak {}\n", bgimage_last_decode_peak());
            if rc == 0 {
                let mut o = BgPicture::default();
                if bgimage_normalize(&pic, 41, 53, &mut o) == 0 {
                    dump_pic(out, "  fit41x53", &o);
                }
            }
        }
    }
}

fn put(path: &str, text: &[u8]) {
    std::fs::write(path, text).unwrap();
}

fn dump_ix(out: &mut String, ix: &BgIndex) {
    w!(out, "  n={}", ix.e.len());
    for e in &ix.e {
        w!(out, " {:06x}={:016x}", e.id, e.key);
    }
    w!(out, "\n");
}

fn sec_index(out: &mut String) {
    let root = root();
    let dir = format!("{}/ixdir", root);
    let _ = std::fs::create_dir(&dir);
    let path = format!("{}/index.ini", dir);
    let _ = std::fs::remove_file(&path);
    let mut ix = BgIndex::default();
    w!(out, "load none {}\n", bgindex_load(&dir, &mut ix));
    dump_ix(out, &ix);
    bgindex_free(&mut ix);
    let texts: [&[u8]; 9] = [
        b"# comment\n000010=00000000000000aa\n000002=00000000000000bb\n",
        b"\n\n  5 = 3\n5=3\n05=ff\n0x1f=abc\n0X20=ABCDEF\n",
        b"1=10\n2=10\n3=20\n1=30\n",
        b"000000=1\n1000000=2\nffffff=3\n1000001=4\n",
        b"-1=5\n+7=6\n8=-1\n9=zz\n10=\nA=1g\n",
        b"nonsense\n=5\n1==5\n\r\n1=5\r\n2=6\r\n",
        b"1=ffffffffffffffffffff\n2=18446744073709551616\n3=0x\n4=0x1\n",
        b"1=2\n\x002=3\n",
        b"ffffffff=1\n100000000=2\n-ffffffff=3\n",
    ];
    let lens = [0usize, 0, 0, 0, 0, 0, 0, 9, 0];
    for (i, t) in texts.iter().enumerate() {
        let n = if lens[i] != 0 { lens[i] } else { t.len() };
        put(&path, &t[..n]);
        let rc = bgindex_load(&dir, &mut ix);
        w!(out, "load text {} rc {}\n", i, rc);
        dump_ix(out, &ix);
        if rc == 0 {
            let sv = bgindex_save(&dir, &ix);
            let b = bgimage_read(&path, 1 << 20).unwrap();
            w!(out, "  save {} bytes {} fnv {:016x}\n", sv, b.len(), fnv(&b));
            out.push_str(&String::from_utf8_lossy(&b));
        }
        bgindex_free(&mut ix);
    }
    // claim
    let _ = std::fs::remove_file(&path);
    let mut f = String::new();
    for id in 1..=3u32 {
        bgimage_file(&dir, id, &mut f, 1000);
        put(&f, b"x");
    }
    bgimage_file(&dir, 0x20, &mut f, 1000);
    put(&f, b"x");
    bgindex_load(&dir, &mut ix);
    let keys: [u64; 11] = [
        0x0,
        0x1000000000000001,
        0x0,
        0xff,
        0xabcdef0123456789,
        0x20,
        0x21,
        0xfffffe,
        0xffffff,
        0xffffffff,
        0xabcdef0123456789,
    ];
    for k in keys {
        let mut id = 0u32;
        let mut added = false;
        let rc = bgindex_claim(&mut ix, &dir, k, &mut id, &mut added);
        w!(out, "claim {:016x} rc {} id {:06x} added {}\n", k, rc, id, added as i32);
    }
    dump_ix(out, &ix);
    w!(out, "save {}\n", bgindex_save(&dir, &ix));
    let mut got = 0;
    let a = bgindex_find(&ix, 0xabcdef0123456789, &mut got) as i32;
    let b = bgindex_find(&ix, 12345, &mut got) as i32;
    w!(out, "find {} {}\n", a, b);
    bgindex_free(&mut ix);
    // unreadable
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let rc = bgindex_load(&dir, &mut ix);
    w!(out, "load unreadable {} n={}\n", rc, ix.e.len());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let rc = bgindex_load(&dir, &mut ix);
    w!(out, "load readable {} n={}\n", rc, ix.e.len());
    bgindex_free(&mut ix);
}

fn sec_files(out: &mut String) {
    let root = root();
    for &sz in &[0usize, 1, 7, 8, 9, 65535, 65536, 65537, 131072, 200003] {
        let path = format!("{}/hf{}.bin", root, sz);
        let mut g = Gen(31 + sz as u32);
        let d: Vec<u8> = (0..sz).map(|_| g.next() as u8).collect();
        put(&path, &d);
        let mut h = 0u64;
        let mut len = 0usize;
        let rc = bgimage_content_hash_file(&path, &mut h, &mut len);
        w!(out, "hashfile {} rc {} len {} same {}\n", sz, rc, len, (h == bgimage_content_hash(&d)) as i32);
        let r = bgimage_read(&path, sz);
        w!(
            out,
            "  read max=size {} {} same {}\n",
            r.is_some() as i32,
            r.as_ref().map_or(0, |r| r.len()),
            r.as_ref().is_some_and(|r| *r == d) as i32
        );
        if sz > 0 {
            let r = bgimage_read(&path, sz - 1);
            w!(out, "  read max=size-1 {} errno {}\n", r.is_some() as i32, if r.is_some() { 0 } else { errno() });
        }
    }
    let mut h = 0u64;
    let mut len = 0usize;
    w!(out, "hashfile missing {}\n", bgimage_content_hash_file("/nonexistent/x.png", &mut h, &mut len));
    w!(out, "hashfile dir {}\n", bgimage_content_hash_file(&root, &mut h, &mut len));
    let r = bgimage_read("/nonexistent/x.png", 100);
    w!(out, "read missing {} errno {}\n", r.is_some() as i32, if r.is_some() { 0 } else { errno() });
    let r = bgimage_read(&root, 100);
    w!(out, "read dir {} errno {}\n", r.is_some() as i32, if r.is_some() { 0 } else { errno() });
}

fn paper(id: u32) -> PaperKind {
    PaperKind(0x50000000 | id)
}

fn render_dump(out: &mut String, tag: &str, c: &Canvas) {
    w!(out, "{} {}x{} fnv {:016x}\n", tag, c.w, c.h, fnv(&c.px[..(c.stride * c.h) as usize]));
}

fn sec_render(out: &mut String) {
    let mut dir = String::new();
    check!(bgimage_dir("nb1", &mut dir, 700) == 0);
    std::fs::create_dir_all(&dir).unwrap();
    let mut f = String::new();
    let pics = [
        (0x123u32, 40, 30, 1),
        (0x124, 40, 30, 1),
        (0x201, 8, 6, 1),
        (0x202, 8, 6, 1),
        (0x203, 8, 6, 1),
        (0x204, 8, 6, 1),
        (0x205, 8, 6, 1),
        (0x400, 25, 50, 3),
    ];
    for (i, pc) in pics.iter().enumerate() {
        let pic = mkpic(pc.1, pc.2, pc.3, 3000 + i as u32);
        bgimage_file(&dir, pc.0, &mut f, 1000);
        w!(out, "write png {:06x} {}\n", pc.0, bgimage_write_png(&f, &pic));
        let b = bgimage_read(&f, 1 << 20).unwrap();
        w!(out, "  file {} fnv {:016x}\n", b.len(), fnv(&b));
    }
    bgimage_file(&dir, 0x300, &mut f, 1000);
    put(&f, b"this is not a png at all, really");
    bgimage_file(&dir, 0x301, &mut f, 1000);
    put(&f, b"\x89PNG\r\n\x1a\n");
    let mut c = Canvas::default();
    canvas_init(&mut c, 40, 30);
    let k = paper(0x123);
    w!(out, "undrawn 123 {} (before)\n", bgimage_undrawn("nb1", 0x123) as i32);
    bgimage_render(&mut c, "nb1", k, Rect { x: 0, y: 0, w: 40, h: 30 });
    render_dump(out, "render same size", &c);
    let u = bgimage_undrawn("nb1", 0x123) as i32;
    w!(out, "undrawn 123 {} (after) missing {}\n", u, bgimage_take_missing() as i32);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 40, h: 30 }, 17);
    bgimage_render(&mut c, "nb1", k, Rect { x: 5, y: 7, w: 20, h: 10 });
    render_dump(out, "render region", &c);
    canvas_set_clip(&mut c, Rect { x: 10, y: 10, w: 10, h: 10 });
    bgimage_render(&mut c, "nb1", k, Rect { x: 0, y: 0, w: 40, h: 30 });
    render_dump(out, "render clipped", &c);
    for s in [(60, 90), (100, 40), (40, 31), (13, 9)] {
        canvas_init(&mut c, s.0, s.1);
        bgimage_render(&mut c, "nb1", k, Rect { x: 0, y: 0, w: s.0, h: s.1 });
        render_dump(out, &format!("render fitted {}x{}", s.0, s.1), &c);
        let mut r = Canvas::default();
        canvas_init(&mut r, s.0, s.1);
        canvas_fill(&mut r, Rect { x: 0, y: 0, w: s.0, h: s.1 }, 99);
        bgimage_render(&mut r, "nb1", k, Rect { x: 3, y: 4, w: s.0 / 2, h: s.1 / 2 });
        render_dump(out, "  region", &r);
        let mut agree = 1;
        let g = rect_intersect(Rect { x: 3, y: 4, w: s.0 / 2, h: s.1 / 2 }, Rect { x: 0, y: 0, w: s.0, h: s.1 });
        for y in g.y..g.y + g.h {
            for x in g.x..g.x + g.w {
                agree &= (r.px[(y * r.stride + x) as usize] == c.px[(y * c.stride + x) as usize]) as i32;
            }
        }
        w!(out, "  region agrees {}\n", agree);
    }
    canvas_init(&mut c, 25, 50);
    bgimage_render(&mut c, "nb1", paper(0x400), Rect { x: 0, y: 0, w: 25, h: 50 });
    render_dump(out, "render rgb", &c);
    canvas_init(&mut c, 40, 30);
    let all = Rect { x: 0, y: 0, w: 40, h: 30 };
    bgimage_render(&mut c, "nb1", paper(0x999), all);
    render_dump(out, "render missing", &c);
    let a = bgimage_take_missing() as i32;
    let b = bgimage_take_missing() as i32;
    w!(out, "take missing {} then {}\n", a, b);
    bgimage_render(&mut c, "nb1", paper(0x999), all);
    w!(out, "again missing {}\n", bgimage_take_missing() as i32);
    bgimage_render(&mut c, "nb1", paper(0x300), all);
    bgimage_render(&mut c, "nb1", paper(0x301), all);
    bgimage_render(&mut c, "nb1", paper(0x300), all);
    w!(out, "bad pngs missing {}\n", bgimage_take_missing() as i32);
    bgimage_render(&mut c, "", paper(0x123), all);
    w!(out, "empty nb missing {}\n", bgimage_take_missing() as i32);
    bgimage_render(&mut c, "nb1", PaperKind(4), all);
    w!(out, "not an image paper missing {}\n", bgimage_take_missing() as i32);
    bgimage_render(&mut c, "nb-nope", paper(0x123), all);
    w!(out, "other notebook missing {}\n", bgimage_take_missing() as i32);
    bgimage_render(&mut c, "nb1", paper(0x123), Rect { x: 100, y: 100, w: 4, h: 4 });
    w!(out, "empty region missing {}\n", bgimage_take_missing() as i32);
    let g0 = bgimage_generation();
    bgimage_retry_missing();
    w!(out, "generation +{}\n", bgimage_generation().wrapping_sub(g0));
    bgimage_render(&mut c, "nb1", paper(0x999), all);
    let pic = mkpic(40, 30, 1, 5555);
    bgimage_file(&dir, 0x999, &mut f, 1000);
    bgimage_write_png(&f, &pic);
    w!(out, "take {}\n", bgimage_take_missing() as i32);
    bgimage_render(&mut c, "nb1", paper(0x999), all);
    w!(out, "still missing {}\n", bgimage_take_missing() as i32);
    bgimage_retry_missing();
    bgimage_render(&mut c, "nb1", paper(0x999), all);
    w!(out, "after retry missing {}\n", bgimage_take_missing() as i32);
    render_dump(out, "render made", &c);
    bgimage_forget(None);
    canvas_init(&mut c, 8, 6);
    let small = Rect { x: 0, y: 0, w: 8, h: 6 };
    for id in 0x201..=0x205u32 {
        bgimage_render(&mut c, "nb1", paper(id), small);
        w!(out, "drew {:x} undrawn:", id);
        for q in 0x201..=0x205u32 {
            w!(out, " {}", bgimage_undrawn("nb1", q) as i32);
        }
        w!(out, "\n");
        if id == 0x203 {
            bgimage_render(&mut c, "nb1", paper(0x201), small);
        }
    }
    bgimage_forget(Some("nb1"));
    w!(out, "forgot:");
    for q in 0x201..=0x205u32 {
        w!(out, " {}", bgimage_undrawn("nb1", q) as i32);
    }
    w!(out, "\n");
    let a = bgimage_undrawn("nb1", 0x7777) as i32;
    let b = bgimage_undrawn("", 0x201) as i32;
    w!(out, "undrawn missing file {}, bad nb {}\n", a, b);
    for id in 0x500..0x500 + 20u32 {
        bgimage_render(&mut c, "nb1", paper(id), small);
    }
    for id in 0x500..0x500 + 20u32 {
        bgimage_render(&mut c, "nb1", paper(id), small);
    }
    bgimage_forget(Some("nb1"));
    bgimage_render(&mut c, "nb1", paper(0x500), small);
}

// What the probe printed for one of its sections.
fn golden(name: &str) -> Vec<&'static str> {
    let all: &'static str = include_str!("fixtures/bgimage_probe_c.txt");
    let mut lines = all.lines().skip_while(|l| *l != format!("## {name}"));
    lines.next();
    lines.take_while(|l| !l.starts_with("## ")).collect()
}

fn compare(name: &str, got: &str) {
    let want = golden(name);
    let got: Vec<&str> = got.lines().collect();
    check!(!want.is_empty());
    check!(got.len() == want.len());
    let mut bad = 0;
    for (g, w) in got.iter().zip(want.iter()) {
        check!(g == w);
        if g != w {
            bad += 1;
            if bad <= 12 {
                eprintln!("  rust {g}\n  c    {w}");
            }
        }
    }
}

fn keys() {
    let mut out = String::new();
    sec_keys(&mut out);
    compare("keys", &out);
}
fn normalize() {
    let mut out = String::new();
    sec_normalize(&mut out);
    compare("normalize", &out);
}
fn orient() {
    let mut out = String::new();
    sec_orient(&mut out);
    compare("orient", &out);
}
fn exif() {
    let mut out = String::new();
    sec_exif(&mut out);
    compare("exif", &out);
}
fn decode() {
    let mut out = String::new();
    sec_decode(&mut out);
    compare("decode", &out);
}
fn index() {
    let mut out = String::new();
    sec_index(&mut out);
    compare("index", &out);
}
fn files() {
    let mut out = String::new();
    sec_files(&mut out);
    compare("files", &out);
}
fn render() {
    let mut out = String::new();
    sec_render(&mut out);
    compare("render", &out);
}

#[test]
fn test_bgimage() {
    let _ = std::fs::remove_dir_all(root());
    std::fs::create_dir_all(root()).unwrap();
    run!(keys);
    run!(normalize);
    run!(orient);
    run!(exif);
    run!(decode);
    run!(index);
    run!(files);
    let _ = std::fs::remove_dir_all(root());
    done();
}

// Drawing finds a picture through store_notebook_dir: this one has a store root.
#[test]
fn test_bgimage_render() {
    let _ = std::fs::remove_dir_all(root());
    check!(store_init(&root()) == 0);
    run!(render);
    let _ = std::fs::remove_dir_all(root());
    done();
}
