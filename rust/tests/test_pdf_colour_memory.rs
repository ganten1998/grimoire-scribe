//! The notebook PDF of Scribe-sized colour pages (1860 x 2480, each on a picture of its own as an imported
//! PDF's pages are, and two sharing one as a colour template's do) mirrors within the mirror child's margin.
//! Its own test binary, so nothing else this process did weighs on the numbers. Measured as the growth of the
//! live malloc bytes while the mirror runs (macOS: the Kindle's musl maps every big block by itself, so its
//! address space follows the live bytes), and in a forked child as bgjob runs it, with its peak resident
//! memory; on Linux that child also carries the real `RLIMIT_AS` a mirror child gets, so a page that needs
//! more than the margin fails there (the CI job `sandbox` runs the same page under the cap as root).

use grimoire::prelude::*;
use std::path::PathBuf;

const W: i32 = 1860;
const H: i32 = 2480;
const MIB: u64 = 1 << 20;

/// What a mirror of colour pages on pictures of their own may take: the picture's decode (its file, the
/// compressed data stb_image gathers, the inflated rows, the pixels: about 50 MiB for a 13 MB photo-like
/// PNG), then one RGB copy of the page (13.8 MB) beside the compressor's output. Measured on the Mac
/// (2026-11-09): 51 MiB for the photo-like pages, 27 for the planner's; 0c26d912 took 81 and 53 (two
/// decodes a page and a kept copy of every picture), and 72 and 40 on one picture (now 69 and 40).
/// Since issue 79 (2026-11-13) the compressor's output is in pieces (zpieces.rs), not stb's one block of
/// 24 MiB for 13.9 MB of data: 50 MiB for the photo-like pages and 56 on one picture, and the same where
/// every large realloc moves its block (it was 63 and 81 there: the block was held twice while it grew).
const ONE_COPY_BUDGET: u64 = 64 * MIB;
/// Pages on one picture keep a copy of it for the next page (one decode for all): a second RGB copy.
const TWO_COPIES_BUDGET: u64 = 80 * MIB;

fn root() -> PathBuf {
    let root = PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("pdf-colour-memory-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn peak_rss() -> u64 {
    // SAFETY: rusage is plain data; getrusage writes one into ru.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    if cfg!(target_vendor = "apple") {
        ru.ru_maxrss as u64
    } else {
        ru.ru_maxrss as u64 * 1024
    }
}

/// The bytes malloc has handed out and not had back, over every zone (macOS; 0 elsewhere).
#[cfg(target_vendor = "apple")]
fn live_bytes() -> u64 {
    #[repr(C)]
    struct MallocStats {
        blocks_in_use: u32,
        size_in_use: usize,
        max_size_in_use: usize,
        size_allocated: usize,
    }
    extern "C" {
        fn malloc_zone_statistics(zone: *mut libc::c_void, stats: *mut MallocStats);
    }
    let mut st = MallocStats { blocks_in_use: 0, size_in_use: 0, max_size_in_use: 0, size_allocated: 0 };
    // SAFETY: malloc_zone_statistics fills the struct we own; a null zone means all zones.
    unsafe { malloc_zone_statistics(std::ptr::null_mut(), &mut st) };
    st.size_in_use as u64
}
#[cfg(not(target_vendor = "apple"))]
fn live_bytes() -> u64 {
    0
}

/// The growth of the live malloc bytes at its sampled peak while f runs.
fn peak_of<R>(f: impl FnOnce() -> R) -> (R, u64) {
    let live0 = live_bytes();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let top = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(live0));
    let (s2, t2) = (stop.clone(), top.clone());
    let sampler = std::thread::spawn(move || {
        while !s2.load(std::sync::atomic::Ordering::Relaxed) {
            t2.fetch_max(live_bytes(), std::sync::atomic::Ordering::Relaxed);
            std::hint::spin_loop();
        }
    });
    let r = f();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = sampler.join();
    (r, top.load(std::sync::atomic::Ordering::Relaxed).saturating_sub(live0))
}

/// kind 1: a planner-like colour page (flat tints, lines); 2: a photo-like colour page (a gradient with five
/// bits of noise a channel: its PNG hardly compresses).
fn picture(path: &str, kind: i32, seed0: u32) {
    let mut px = vec![255u8; (W * H * 3) as usize];
    let mut seed = seed0;
    let mut rnd = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 24) as u8
    };
    for y in 0..H {
        for x in 0..W {
            let i = ((y * W + x) * 3) as usize;
            if kind == 1 {
                if y % 120 < 3 || x % 240 < 2 {
                    px[i..i + 3].copy_from_slice(&[90, 120, 200]);
                } else if y < 160 {
                    px[i..i + 3].copy_from_slice(&[250, 210, 120]);
                }
            } else {
                px[i] = ((x * 200 / W) as u8).wrapping_add(rnd() & 31);
                px[i + 1] = ((y * 200 / H) as u8).wrapping_add(rnd() & 31);
                px[i + 2] = (((x + y) * 100 / (W + H)) as u8).wrapping_add(rnd() & 31);
            }
        }
    }
    assert_eq!(bgimage_write_png(path, &BgPicture { px, w: W, h: H, ch: 3 }), 0);
}

/// Three Scribe pages, each with a stroke: on pictures of their own (as an imported PDF's pages are), or all
/// three on one picture (a colour template's: the kept copy's case).
fn book(name: &str, kind: i32, shared: bool) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create(name, "", PAPER_BLANK, &mut nb), 0);
    for i in 1..3 {
        assert_eq!(store_add_page(&mut nb, i, PAPER_BLANK), 0);
    }
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    assert_eq!(mkdir_p(&bg), 0);
    for i in 0..3 {
        let id = 0x30 + if shared { 0 } else { i as u32 };
        let mut f = String::new();
        bgimage_file(&bg, id, &mut f, 800);
        if !shared || i == 0 {
            picture(&f, kind, 777 + i as u32);
        }
        let mut p = page_new(W, H, paper_image(PAPER_ROLE_DOCUMENT, id));
        let mut tp = ToolParams::default();
        tool_params_default(TOOL_PENCIL, &mut tp);
        let mut s = stroke_new(TOOL_PENCIL, &tp, 7 + i as u32);
        for k in 0..30 {
            let q = PenSample { x: 200.0 + 20.0 * k as f32, y: 600.0 + 6.0 * (k % 5) as f32, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: k as u32 * 7 };
            stroke_append(&mut s, &q, 300.0);
        }
        page_add(&mut p, s);
        assert_eq!(store_save_page(&nb, i, &p), 0);
    }
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    b
}

/// The mirror in a forked child, as bgjob runs it (on Linux under the mirror child's real cap): its exit
/// status (0 done, 2 the export failed) and the growth of its peak resident memory.
fn mirror_in_child(nb: &NotebookInfo, dir: &str, note: &str) -> (i32, u64) {
    let _ = std::fs::remove_file(note);
    // SAFETY: fork; the child only exports, writes one number and _exits.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        #[cfg(target_os = "linux")]
        child_caps_apply(child_margin(BgKind::MirrorPdf));
        let start = peak_rss();
        let rc = export_mirror_pdf(nb, 300.0, dir);
        let _ = std::fs::write(note, peak_rss().saturating_sub(start).to_string());
        // SAFETY: _exit ends the child at once.
        unsafe { libc::_exit(if rc == 0 { 0 } else { 2 }) };
    }
    let mut st = 0;
    // SAFETY: waitpid on the child just forked.
    unsafe { libc::waitpid(pid, &mut st, 0) };
    let grew: u64 = std::fs::read_to_string(note).ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    (if libc::WIFEXITED(st) { libc::WEXITSTATUS(st) } else { -1 }, grew)
}

#[test]
fn scribe_sized_colour_pages_mirror_within_the_mirror_childs_margin() {
    let r = root();
    assert_eq!(store_init(r.to_str().unwrap()), 0);
    store_set_device(300.0, W, H);
    let dir = format!("{}/PDFs", r.display());
    let note = format!("{}/grew.txt", r.display());
    let _ = peak_of(|| 0); // (the sampler's own first cost, outside the numbers)
    let margin = child_margin(BgKind::MirrorPdf);
    let mut worst = 0;
    let mut over = Vec::new();
    for (name, kind, shared) in [("Planner", 1, false), ("Photo", 2, false), ("Planner on one picture", 1, true), ("Photo on one picture", 2, true)] {
        let nb = book(name, kind, shared);
        bgimage_forget(None);
        let (rc, rss) = mirror_in_child(&nb, &dir, &note);
        std::fs::remove_dir_all(pdf_cache_root()).unwrap();
        bgimage_forget(None);
        let (rc2, live) = peak_of(|| export_mirror_pdf(&nb, 300.0, &dir));
        println!(
            "MEMORY {name}: live malloc bytes grew {:.1} MiB; a forked child's peak resident memory grew {:.1} MiB; the margin is {} MiB",
            live as f64 / MIB as f64,
            rss as f64 / MIB as f64,
            margin / MIB
        );
        if (rc, rc2) != (0, 0) {
            over.push(format!("{name}: the mirror failed (child {rc}, here {rc2}; in the child: under the mirror's cap on Linux)"));
        }
        // pages on pictures of their own hold one RGB copy at a time; pages on one picture a second (the kept one)
        let budget = if shared { TWO_COPIES_BUDGET } else { ONE_COPY_BUDGET };
        // (the per-copy budgets are what the owner's Mac measured: macOS's own allocator reports more on older
        // systems, 72 and 84 MiB on GitHub's macOS runner for 51 and 69 here, so they are printed, not held; what
        // the device needs is the margin below, with headroom. Those two readings, and a 96 that failed the
        // headroom once, were from before issue 79: with stb's output block in them, moved or not)
        if cfg!(target_vendor = "apple") && live > budget {
            println!("NOTE {name}: {} MiB, over the {} MiB of {} RGB cop{} of the page measured on the owner's Mac", live / MIB, budget / MIB, if shared { "two" } else { "one" }, if shared { "ies" } else { "y" });
        }
        worst = worst.max(live);
        assert_eq!(store_delete(nb.id.as_str()), 0);
    }
    if cfg!(target_vendor = "apple") && worst + 32 * MIB > margin {
        over.push(format!("the margin ({} MiB) keeps no 32 MiB of headroom over the measured {} MiB", margin / MIB, worst / MIB));
    }
    assert!(over.is_empty(), "{over:#?}");
    let _ = std::fs::remove_dir_all(&r);
}
