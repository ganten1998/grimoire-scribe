// test_fontcheck.rs: your own fonts (2.1.0-beta.2, lane L10; DESIGN.md "Your own fonts", 3.7, 4.2 and 9 "L10").
// Every font goes through a real worker, a fresh exec of the headless runner (GRIMOIRE_WORK_EXE), as in the app,
// and through the real background job where a test is about the folder. The fonts are made here (tests/fontfix):
// good ones, and the malformed-font corpus, which is defensive test data for Grimoire's own check. The Linux
// sandbox steps need root and run in the CI job (test_sandbox_linux.rs).
#![cfg(feature = "sim")]
mod fontfix;
mod testlib;
use fontfix::*;
use grimoire::fontcheck::*;
use grimoire::prelude::*;
use grimoire::userfonts::*;
use std::os::unix::fs::MetadataExt;
use testlib::*;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_grimoire-headless")
}

const HOOKS: [&str; 13] = [
    "GRIMOIRE_STATE",
    "GRIMOIRE_STATE_FREE_BYTES",
    "GRIMOIRE_FONT_GLYPH_US",
    "GRIMOIRE_FONT_TEST_SLOW",
    "GRIMOIRE_FONT_TEST_CRASH",
    "GRIMOIRE_FONT_TEST_HANG",
    "GRIMOIRE_FONT_TEST_TRAP",
    "GRIMOIRE_FONT_WATCHDOG_MS",
    "GRIMOIRE_FONT_SLICE_MS",
    "GRIMOIRE_FONT_JOB_MS",
    "GRIMOIRE_FONT_MEM_AVAILABLE",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
}

/// A fresh data root with an empty fonts folder; the store looks at it from now on.
fn root(name: &str) -> String {
    let d = format!("{}/fontcheck-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(format!("{d}/fonts")).unwrap();
    store_init(&d);
    d
}

fn put(root: &str, rel: &str, bytes: &[u8]) {
    let p = format!("{root}/fonts/{rel}");
    std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// Runs the job in this process (the writer's side as the background child runs it): a job checks one font and
/// says 5 when more are left, as the app queues the next one; this runs them all and gives the last one's code.
fn job() -> i32 {
    loop {
        let rc = bg_font_check("", 300.0);
        if rc != 5 {
            return rc;
        }
    }
}

fn asset(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../assets/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn passed(out: FontOut) -> (FaceFacts, Vec<u8>, i32, i32) {
    match out {
        FontOut::Passed { facts, preview, w, h, .. } => (facts, preview, w, h),
        other => panic!("expected a pass, got {other:?}"),
    }
}

/// A pass with the walk's figures: what the check learned, the preview, how many of the skipped glyphs were
/// too slow to draw, the dearest draw, and the letters' and digits' draws in all.
fn passed_timed(out: FontOut) -> (FaceFacts, Vec<u8>, u32, u64, u64) {
    match out {
        FontOut::Passed { facts, preview, slow, slowest_us, letters_us, .. } => (facts, preview, slow, slowest_us, letters_us),
        other => panic!("expected a pass, got {other:?}"),
    }
}

fn refused(out: FontOut) -> Refuse {
    match out {
        FontOut::Refused(r) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn state_of(list: &[UserFont], rel: &str) -> &'static str {
    let mut f = list.iter().find(|f| f.rel == rel).unwrap_or_else(|| panic!("{rel} is not listed")).clone();
    userfont_resolve(&mut f);
    f.state_name()
}

// The listing alone, no job: every file's state as the cache has it (a scan would queue the check).
fn listed() -> Vec<UserFont> {
    let mut l = userfonts_scan(300.0);
    wait_job();
    userfonts_refresh(&mut l);
    l
}

/// Waits for the FontCheck job a scan queued (if one runs): its exit status, or 0 when none ran.
fn wait_job() -> i32 {
    let t0 = std::time::Instant::now();
    let mut code = 0;
    while bg_busy() {
        let mut kind = -1;
        bg_poll_kind(Some(&mut kind));
        if kind == BgKind::FontCheck as i32 {
            code = bg_last_exit();
            if code == 5 {
                // (one font a job: the app's next one, behind everything else)
                bg_queue(BgKind::FontCheck, Some(""), Some("next"), 300.0, false);
            }
        }
        assert!(t0.elapsed().as_secs() < 120, "the font check did not end");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    code
}

// Good fonts pass: the test face as TrueType, as a Bold, with a weight axis, as CFF (cubic curves, subroutines
// within the format's depth), and three files Grimoire ships. What the check learns of each is what the file
// says; the preview is the family's name drawn in the font, black on white, on the row's own height.
fn good_fonts_pass() {
    clean_env();
    let (f, pv, w, h) = passed(check_font(&ttf(&test_font("Regular", 400)), 44));
    check!(f.family == "Grimoire Test" && f.style == "Regular" && f.weight == 400 && !f.italic);
    check!((f.upem, f.ascent, f.descent) == (1000, 800, -200) && f.sane());
    check!(f.mapped == 95 && f.skip.is_empty() && f.wght.is_none() && f.bold_at == 0);
    check!(h == 63 && w > 100 && w <= 880 && pv.len() == (w * h) as usize);
    // (ink on white: some black, mostly white, and a white frame)
    let dark = pv.iter().filter(|&&p| p < 64).count();
    check!(dark > 500 && dark < pv.len() / 2);
    check!(pv[..w as usize].iter().all(|&p| p == 255));
    // every UI language has letters this ASCII face lacks (the ellipsis, the accents)
    check!(f.lacks & 0b111 == 0b111);

    let (b, ..) = passed(check_font(&ttf(&test_font("Bold", 700)), 44));
    check!(b.weight == 700 && b.style == "Bold" && b.family == f.family);

    let (v, ..) = passed(check_font(&ttf(&test_font_variable()), 44));
    check!(v.wght == Some((400, 400, 700)) && v.bold_at == 700 && v.skip.is_empty());

    let (c, ..) = passed(check_font(&otf(&test_font("Regular", 400), 0, false), 44));
    check!(c.mapped == 95 && c.skip.is_empty() && (c.ascent, c.descent) == (800, -200));
    let (c2, ..) = passed(check_font(&otf(&test_font("Regular", 400), 2, false), 44));
    check!(c2.skip.is_empty());

    for (file, family, weight) in [("NotoSans-Regular.ttf", "Noto Sans", 400), ("Literata-Bold.ttf", "Literata", 700), ("AtkinsonHyperlegibleNext-Regular.ttf", "Atkinson Hyperlegible Next", 400)] {
        let (n, pv, w, h) = passed(check_font(&asset(file), 44));
        check!(n.family == family && n.weight == weight && n.skip.is_empty() && n.sane());
        check!(n.lacks & 0b011 == 0); // English and Spanish are whole in it
        check!(n.cover[0] >= 94 && h == 63 && pv.iter().filter(|&&p| p < 64).count() > 300 && w > 100);
        check!((860..=1250).contains(&n.scale_pm));
    }
    // the preview follows the screen's text size
    let (_, _, _, h2) = passed(check_font(&ttf(&test_font("Regular", 400)), 30));
    check!(h2 == 46);
    // a hairline font (stems 8 units of 1000 wide) is thin, not blank: it passes
    check!(passed(check_font(&ttf(&thin(8, 1000)), 44)).0.skip.is_empty());
}

// The malformed-font corpus: each gets a verdict, and the worker never dies on one. A font cut short anywhere, a
// cmap that points past the end, composite glyphs that include each other, CFF subroutines nested past the
// format's ten, a symbol cmap, a web font, letters that draw nothing, bytes flipped at random.
fn the_corpus_gets_verdicts_never_a_crash() {
    clean_env();
    let good = ttf(&test_font("Regular", 400));
    // Truncated: a cut anywhere before the last table the check reads (the tables come in tag order: the names
    // and the PostScript table are last) is refused; a cut inside those two leaves a font that still draws,
    // which may pass. Through a worker for a spread of cuts, in this process for many more.
    let needed = table(&good, b"name").0;
    check!(matches!(check_font(&good[..0], 44), FontOut::Refused(Refuse::Unreadable)));
    for cut in [1usize, 3, 4, 11, 12, 100, 172, 173, 300, 700, 2000, 9000, needed - 1, needed + 40, good.len() - 20, good.len() - 1] {
        let out = check_font(&good[..cut], 44);
        if cut < needed {
            check!(matches!(out, FontOut::Refused(Refuse::Unreadable) | FontOut::Refused(Refuse::NoLatin)));
        } else {
            check!(matches!(out, FontOut::Passed { .. } | FontOut::Refused(_)));
        }
    }
    for cut in (1..good.len()).step_by(37) {
        match facts_in_process(&good[..cut]) {
            Ok(_) => check!(cut >= needed && glyphs_failing_in_process(&good[..cut]).is_ok_and(|g| g.is_empty())),
            Err(_) => check!(cut < good.len()),
        }
    }
    let cff = otf(&test_font("Regular", 400), 2, false);
    for cut in (1..cff.len()).step_by(11) {
        if facts_in_process(&cff[..cut]).is_ok() {
            let _ = glyphs_failing_in_process(&cff[..cut]);
        }
    }
    for bytes in [cmap_past_end(), ttf(&symbol_cmap())] {
        check!(refused(check_font(&bytes, 44)) == Refuse::NoLatin);
    }
    check!(refused(check_font(&woff2(), 44)) == Refuse::Format);
    for bytes in [
        ttf(&cyclic_composite(true)),
        ttf(&empty_letters()),
        // (letters of dust, a unit each way: nothing a screen could show)
        ttf(&thin(1, 1)),
        otf(&test_font("Regular", 400), 12, false),
        otf(&test_font("Regular", 400), 200, false),
        ttf(&wild_glyphs(Wild::TooMany)),
        vec![0u8; 5000],
        b"not a font at all, only words".to_vec(),
    ] {
        check!(refused(check_font(&bytes, 44)) == Refuse::Unreadable);
    }
    // A few broken glyphs in a font of many: it passes, and those glyphs come from Terminess.
    let ring = cyclic_composite(false);
    let (f, ..) = passed(check_font(&ttf(&ring), 44));
    check!(f.skip == vec![glyph_index(&ring, '\u{a4}') as u16] && f.mapped == 191);
    let far = wild_glyphs(Wild::Far);
    check!(passed(check_font(&ttf(&far), 44)).0.skip == vec![glyph_index(&far, '\u{a7}') as u16]);
    let dense = wild_glyphs(Wild::Dense);
    check!(passed(check_font(&ttf(&dense), 44)).0.skip == vec![glyph_index(&dense, '\u{b6}') as u16]);
    let deep = test_font_latin1("Regular", 400);
    check!(passed(check_font(&otf(&deep, 12, true), 44)).0.skip == vec![glyph_index(&deep, '~') as u16]);
    // A line the tables put ten ems up: the check takes what the letters reach.
    let (tall, ..) = passed(check_font(&ttf(&tall_line()), 44));
    check!(tall.ascent == 700 && tall.descent == -175);
    // Bytes flipped at random (the same ones every run): a verdict or a pass in this process, never a panic;
    // and through a worker for some of them, which never dies.
    let mut seed = 0x2545F4914F6CDD1Du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let noto = asset("NotoSans-Regular.ttf");
    for (base, rounds) in [(&good, 300), (&noto, 300), (&otf(&test_font("Regular", 400), 2, false), 200), (&ttf(&test_font_variable()), 200)] {
        for round in 0..rounds {
            let mut b = base.clone();
            for _ in 0..(1 + round % 12) {
                let at = (next() % b.len() as u64) as usize;
                b[at] = next() as u8;
            }
            if facts_in_process(&b).is_ok() {
                let _ = glyphs_failing_in_process(&b);
            }
            if round % 40 == 0 {
                let out = check_font(&b, 44);
                check!(matches!(out, FontOut::Passed { .. } | FontOut::Refused(_)));
            }
        }
    }
}

// A worker killed on a font marks that font bad and the others go on; the sheet's list says so. A file as big as
// 30 MB (sparse: no byte of it is read) is too big; a web font is refused by its name alone.
fn a_killed_worker_leaves_bad_and_the_rest_go_on() {
    clean_env();
    let r = root("killed");
    put(&r, "A-first.ttf", &ttf(&test_font("Regular", 400)));
    put(&r, "B-dies.ttf", &ttf(&test_font_latin1("Regular", 400)));
    put(&r, "C-last.otf", &otf(&test_font("Regular", 400), 0, false));
    put(&r, "Web.woff2", &woff2());
    put(&r, "notes.txt", b"not a font");
    put(&r, ".hidden.ttf", &ttf(&test_font("Regular", 400)));
    let big = std::fs::File::create(format!("{r}/fonts/Sparse.ttf")).unwrap();
    big.set_len(30 << 20).unwrap();
    drop(big);
    // (glyph 150 is one only the Latin-1 font has: its worker dies there)
    std::env::set_var("GRIMOIRE_FONT_TEST_CRASH", "150");
    check!(job() == 0);
    std::env::remove_var("GRIMOIRE_FONT_TEST_CRASH");
    let cache = format!("{r}/fonts/.cache");
    let names: Vec<String> = std::fs::read_dir(&cache).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    check!(names.iter().filter(|n| n.ends_with(".font")).count() == 2);
    check!(names.iter().filter(|n| n.ends_with(".pgm")).count() == 2);
    check!(names.iter().filter(|n| n.ends_with(".bad")).count() == 1);
    check!(!names.iter().any(|n| n.ends_with(".checking") || n.ends_with(".tmp")));
    let l = listed();
    check!(l.iter().map(|f| f.rel.as_str()).collect::<Vec<_>>() == vec!["A-first.ttf", "B-dies.ttf", "C-last.otf", "Sparse.ttf", "Web.woff2"]);
    check!(state_of(&l, "A-first.ttf") == "ready" && state_of(&l, "C-last.otf") == "ready");
    check!(state_of(&l, "B-dies.ttf") == "unreadable");
    check!(state_of(&l, "Sparse.ttf") == "toobig" && state_of(&l, "Web.woff2") == "format");
    // its preview reads back as the picture the check made
    let mut a = l.iter().find(|f| f.rel == "A-first.ttf").unwrap().clone();
    userfont_resolve(&mut a);
    check!(a.family() == "Grimoire Test");
    let pv = userfont_preview(&mut a).map(|c| (c.w, c.h));
    check!(pv.is_some_and(|(w, h)| h == 63 && w > 100));
    // the bad one stays bad at the next look, with no worker started for it
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(job() == 0);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    // until the file changes: another key, checked afresh, and the old marks are swept
    put(&r, "B-dies.ttf", &ttf(&test_font_latin1("Bold", 700)));
    check!(job() == 0);
    let l = listed();
    check!(state_of(&l, "B-dies.ttf") == "ready");
    check!(std::fs::read_dir(&cache).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".bad")).count() == 0);
    // a file that goes takes its cache with it
    std::fs::remove_file(format!("{r}/fonts/C-last.otf")).unwrap();
    check!(job() == 0);
    check!(std::fs::read_dir(&cache).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".font")).count() == 2);
}

// A worker that never ends a glyph is killed by the watchdog: the font took too long, and says so; one that takes
// longer than a font may in all is the same. A font checked in many slices (a fresh worker each, resumed by
// glyph index) comes to the very verdict and preview one slice gives.
fn slow_fonts_and_slices() {
    clean_env();
    let font = ttf(&test_font_many(4000));
    let (one, pv1, w1, h1) = passed(check_font(&font, 44));
    check!(one.mapped == 4095 && one.cover[5] == 4000);
    std::env::set_var("GRIMOIRE_FONT_SLICE_MS", "2");
    let t = std::time::Instant::now();
    let (many, pv2, w2, h2) = passed(check_font(&font, 44));
    eprintln!("    4095 glyphs in slices of 2 ms: {:?}", t.elapsed());
    check!(many == one && pv2 == pv1 && (w2, h2) == (w1, h1));
    // (and a broken glyph late in the font is found by whichever slice reaches it)
    let mut late = test_font_many(4000);
    let last = late.glyphs.len() - 7;
    late.glyphs[last].contours = vec![vec![(0, 0), (0, 30000), (30000, 30000), (30000, 0)]];
    check!(passed(check_font(&ttf(&late), 44)).0.skip == vec![last as u16]);
    std::env::set_var("GRIMOIRE_FONT_JOB_MS", "3");
    check!(matches!(check_font(&font, 44), FontOut::Slow));
    std::env::remove_var("GRIMOIRE_FONT_JOB_MS");
    std::env::remove_var("GRIMOIRE_FONT_SLICE_MS");
    std::env::set_var("GRIMOIRE_FONT_TEST_HANG", "40");
    std::env::set_var("GRIMOIRE_FONT_WATCHDOG_MS", "700");
    let t = std::time::Instant::now();
    check!(matches!(check_font(&ttf(&test_font("Regular", 400)), 44), FontOut::Slow));
    check!(t.elapsed().as_millis() >= 600 && t.elapsed().as_secs() < 20);
    // in the folder: a first timeout is not a verdict (the clock may have jumped after a wake): the font waits a
    // minute and is checked once more; the second timeout is the row's "took too long", and it is not tried again
    let r = root("slow");
    put(&r, "Hangs.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 3);
    let mark = std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "slow"));
    check!(mark.is_some());
    // (the minute's wait is skipped under the test watchdog: the next job tries it again, and the verdict stands)
    let mark = mark.unwrap();
    check!(job() == 0);
    std::env::remove_var("GRIMOIRE_FONT_TEST_HANG");
    std::env::remove_var("GRIMOIRE_FONT_WATCHDOG_MS");
    let l = listed();
    check!(state_of(&l, "Hangs.ttf") == "slow");
    check!(!mark.exists());
}

// Files' preview of a font (2.2, issue #145): the worker opens the face and draws the fixed sample on a white
// picture of the size asked for (two lines, "Aa" and "Abc 123", each centred), without the check's walk; the same
// font gives the same picture, a variable and a CFF one are drawn too. A font the check refuses is refused here the
// same way, a WOFF2 by its format; a worker that never answers is the watchdog's (Slow), one that traps or crashes
// ended (Dead). Sizes outside the protocol's are refused before any worker.
fn a_fonts_sample_is_drawn_by_the_worker() {
    clean_env();
    let (w, h) = (300, 400);
    let good = ttf(&test_font("Regular", 400));
    let px = match font_sample(&good, w, h) {
        Ok(px) => px,
        Err(e) => panic!("the sample: {e:?}"),
    };
    check!(px.len() == (w * h) as usize);
    let dark_rows: Vec<usize> = (0..h as usize).filter(|&y| px[y * w as usize..(y + 1) * w as usize].iter().any(|&v| v < 128)).collect();
    check!(!dark_rows.is_empty());
    // two lines of ink, apart: a run of rows with ink, a gap, another run
    let runs = dark_rows.windows(2).filter(|p| p[1] != p[0] + 1).count() + 1;
    check!(runs == 2);
    // centred: the ink's left and right margins differ by little
    let cols: Vec<usize> = (0..w as usize).filter(|&x| (0..h as usize).any(|y| px[y * w as usize + x] < 128)).collect();
    let (l, r) = (cols[0], w as usize - 1 - cols[cols.len() - 1]);
    check!(l.abs_diff(r) < 40);
    // the same font, the same picture; a variable font and a CFF one are drawn too
    check!(font_sample(&good, w, h).ok() == Some(px.clone()));
    check!(matches!(font_sample(&ttf(&test_font_variable()), w, h), Ok(p) if p.iter().any(|&v| v < 128)));
    check!(matches!(font_sample(&otf(&test_font("Regular", 400), 2, false), w, h), Ok(p) if p.iter().any(|&v| v < 128)));
    // refused as the check refuses
    check!(matches!(font_sample(&woff2(), w, h), Err(FontOut::Refused(Refuse::Format))));
    check!(matches!(font_sample(&ttf(&symbol_cmap()), w, h), Err(FontOut::Refused(Refuse::NoLatin))));
    check!(matches!(font_sample(&ttf(&empty_letters()), w, h), Err(FontOut::Refused(Refuse::Unreadable))));
    check!(matches!(font_sample(b"not a font at all", w, h), Err(FontOut::Refused(_))));
    // sizes the protocol does not take: refused here
    check!(matches!(font_sample(&good, 7, h), Err(FontOut::Refused(_))));
    check!(matches!(font_sample(&good, w, 801), Err(FontOut::Refused(_))));
    // a worker that hangs on the sample: the watchdog's; one that traps or crashes: ended
    std::env::set_var("GRIMOIRE_FONT_TEST_HANG", "0");
    std::env::set_var("GRIMOIRE_FONT_WATCHDOG_MS", "500");
    let t = std::time::Instant::now();
    check!(matches!(font_sample(&good, w, h), Err(FontOut::Slow)));
    check!(t.elapsed().as_millis() >= 400 && t.elapsed().as_secs() < 20);
    std::env::remove_var("GRIMOIRE_FONT_TEST_HANG");
    std::env::remove_var("GRIMOIRE_FONT_WATCHDOG_MS");
    std::env::set_var("GRIMOIRE_FONT_TEST_CRASH", "0");
    check!(matches!(font_sample(&good, w, h), Err(FontOut::Dead(_))));
    std::env::remove_var("GRIMOIRE_FONT_TEST_CRASH");
    // (the check's hooks at glyph 0 never fire there: glyph 0 is no character's)
    std::env::set_var("GRIMOIRE_FONT_TEST_HANG", "0");
    check!(matches!(check_font(&good, 44), FontOut::Passed { .. }));
    std::env::remove_var("GRIMOIRE_FONT_TEST_HANG");
}

// A sandbox step that fails is not the font's fault: nothing is checked, nothing is marked, the job says no
// worker can be had (4), and a worker that cannot start at all is the same.
fn a_failed_sandbox_step_checks_nothing() {
    clean_env();
    let r = root("nosandbox");
    put(&r, "Fine.ttf", &ttf(&test_font("Regular", 400)));
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
    check!(matches!(check_font(&ttf(&test_font("Regular", 400)), 44), FontOut::NoWorker(_)));
    // (the session checks no more fonts: a later font finds no worker either, without one being started)
    std::env::remove_var("GRIMOIRE_WORK_FAIL_STEP");
    check!(grimoire::sandbox::session_broken());
    check!(matches!(check_font(&ttf(&test_font("Regular", 400)), 44), FontOut::NoWorker(_)));
    check!(job() == 4);
    let cache: Vec<String> = std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    check!(cache.is_empty());
}

// The folder: fonts in it and one folder level down, in name order; links, dot-files, deeper folders and other
// files are not fonts of it; at most 200.
fn the_folder_is_listed_by_the_walk() {
    clean_env();
    let r = root("listing");
    let font = ttf(&test_font("Regular", 400));
    put(&r, "Zeta.ttf", &font);
    put(&r, "alpha.TTF", &font);
    put(&r, "Family/Family-Regular.otf", &otf(&test_font("Regular", 400), 0, false));
    put(&r, "Family/Family-Bold.ttf", &ttf(&test_font("Bold", 700)));
    put(&r, "Family/deeper/TooDeep.ttf", &font);
    put(&r, "Font 2.ttf", &font);
    put(&r, "Font 10.ttf", &font);
    put(&r, ".cache-not/x.ttf", &font);
    std::os::unix::fs::symlink(format!("{r}/fonts/Zeta.ttf"), format!("{r}/fonts/Link.ttf")).unwrap();
    std::os::unix::fs::symlink("/etc/hosts", format!("{r}/fonts/Hosts.ttf")).unwrap();
    std::os::unix::fs::symlink(format!("{r}/fonts/Family"), format!("{r}/fonts/LinkedFolder")).unwrap();
    check!(job() == 0);
    let l = listed();
    let rels: Vec<&str> = l.iter().map(|f| f.rel.as_str()).collect();
    check!(rels == vec!["alpha.TTF", "Family/Family-Bold.ttf", "Family/Family-Regular.otf", "Font 2.ttf", "Font 10.ttf", "Zeta.ttf"]);
    check!(l.iter().all(|f| state_of(&l, &f.rel) == "ready"));
    check!(l[0].base() == "alpha" && l[1].base() == "Family-Bold");
    // more files than the folder may hold: the first 200 by name
    let r = root("many");
    for i in 0..205 {
        put(&r, &format!("F{i:03}.woff"), b"wOFF");
    }
    let l = userfonts_scan(300.0);
    check!(l.len() == FONT_FILES_MAX && l[0].rel == "F000.woff" && l[199].rel == "F199.woff");
    check!(userfonts_checking(&l) == 0);
}

// The job waits for a file still being copied (young, or changing): nothing is marked, and it says some files
// wait (3). Checked files are never read again by a later job.
fn a_file_still_being_copied_waits() {
    clean_env();
    std::env::remove_var("GRIMOIRE_IMPORT_STABLE");
    let r = root("copying");
    put(&r, "Fresh.ttf", &ttf(&test_font("Regular", 400)));
    check!(job() == 3);
    check!(std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().flatten().count() == 0);
    // older than the five seconds a copy is given: checked (the job waits out the two seconds between its looks)
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
    std::fs::File::options().write(true).open(format!("{r}/fonts/Fresh.ttf")).unwrap().set_modified(old).unwrap();
    let t = std::time::Instant::now();
    check!(job() == 0);
    check!(t.elapsed().as_millis() >= 1500);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    check!(state_of(&listed(), "Fresh.ttf") == "ready");
}

// The verdict binds the whole file: another file put under the same name, size and time is not the one that
// passed. The UI finds it when it reads the bytes (their hash is not the verdict's), loads nothing, and has the
// file checked again; then the new bytes load. A font that passed loads with its faces: bold by the weight
// axis, else by the Bold file of its family beside it, else none (overprint).
fn a_replaced_file_is_checked_again_and_bold_is_found() {
    clean_env();
    let r = root("replaced");
    let first = ttf(&test_font("Regular", 400));
    put(&r, "Mine.ttf", &first);
    put(&r, "Mine-Bold.ttf", &ttf(&test_font("Bold", 700)));
    put(&r, "Axis.ttf", &ttf(&test_font_variable()));
    let mut lone = test_font("Regular", 400);
    lone.family = "Alone".into();
    put(&r, "Alone.ttf", &ttf(&lone));
    check!(job() == 0);
    check!(userfonts_try_load("Mine.ttf", "") == Ok("a Bold file"));
    check!(userfonts_try_load("Axis.ttf", "") == Ok("its weight axis"));
    check!(userfonts_try_load("Alone.ttf", "") == Ok("overprint"));
    check!(userfonts_try_load("Mine-Bold.ttf", "") == Ok("overprint"));
    // (settings.ini's font_bold names the Bold file by hand)
    check!(userfonts_try_load("Alone.ttf", "Mine-Bold.ttf") == Ok("a Bold file"));
    check!(userfonts_try_load("Gone.ttf", "") == Err(LoadErr::Missing));
    // The same name, size and time, other bytes: one coordinate of one glyph moved.
    let path = format!("{r}/fonts/Mine.ttf");
    let md = std::fs::metadata(&path).unwrap();
    let mut second = first.clone();
    let (glyf, _) = table(&second, b"glyf");
    second[glyf + 300] ^= 0x01;
    check!(second.len() == first.len() && font_hash(&second) != font_hash(&first));
    std::fs::write(&path, &second).unwrap();
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(md.modified().unwrap()).unwrap();
    let now = std::fs::metadata(&path).unwrap();
    check!(now.len() == md.len() && now.mtime() == md.mtime());
    // the list still says ready (the verdict is there), but the load does not take the bytes
    check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Checking));
    // ... and has queued the check again, for that file: once it ran, the verdict is of the new bytes
    check!(wait_job() == 0);
    check!(userfonts_try_load("Mine.ttf", "") == Ok("a Bold file"));
    let l = listed();
    let mut f = l.iter().find(|f| f.rel == "Mine.ttf").unwrap().clone();
    userfont_resolve(&mut f);
    check!(matches!(&f.state, UserFontState::Ready(_, h) if *h == font_hash(&second)));
    // too little memory free: not loaded, and nothing is marked against the font
    std::env::set_var("GRIMOIRE_FONT_MEM_AVAILABLE", "50000000");
    check!(userfonts_try_load("Mine.ttf", "") == Err(LoadErr::Memory));
    std::env::remove_var("GRIMOIRE_FONT_MEM_AVAILABLE");
    check!(userfonts_try_load("Mine.ttf", "") == Ok("a Bold file"));
    // a font the check refused never loads
    put(&r, "Blank.ttf", &ttf(&empty_letters()));
    check!(job() == 0);
    check!(userfonts_try_load("Blank.ttf", "") == Err(LoadErr::Refused));
    check!(state_of(&listed(), "Blank.ttf") == "unreadable");
}

// The fonts the UI holds stay under 32 MB together: a big family's Bold file gives way to overprint; and a
// file over 24 MB is too big whatever it holds (it is never read).
fn the_loaded_fonts_are_capped() {
    clean_env();
    let r = root("caps");
    let mut reg = test_font("Regular", 400);
    reg.family = "Heavy".into();
    let mut bold = test_font("Bold", 700);
    bold.family = "Heavy".into();
    put(&r, "Heavy-Regular.ttf", &ttf_padded(&reg, 17 << 20));
    put(&r, "Heavy-Bold.ttf", &ttf_padded(&bold, 17 << 20));
    put(&r, "Huge.ttf", &ttf_padded(&reg, 25 << 20));
    check!(job() == 0);
    let l = listed();
    check!(state_of(&l, "Heavy-Regular.ttf") == "ready" && state_of(&l, "Heavy-Bold.ttf") == "ready");
    check!(state_of(&l, "Huge.ttf") == "toobig");
    check!(!std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".bad")));
    // 17 MB and 17 MB are more than the UI holds: the regular, and bold by overprint
    check!(userfonts_try_load("Heavy-Regular.ttf", "") == Ok("overprint"));
    check!(userfonts_try_load("Huge.ttf", "") == Err(LoadErr::Refused));
    // a family that fits keeps its Bold file
    put(&r, "Light-Regular.ttf", &ttf_padded(&reg, 2 << 20));
    put(&r, "Light-Bold.ttf", &ttf_padded(&bold, 2 << 20));
    std::fs::remove_file(format!("{r}/fonts/Heavy-Bold.ttf")).unwrap();
    check!(job() == 0);
    check!(userfonts_try_load("Light-Regular.ttf", "") == Ok("a Bold file"));
    let _ = std::fs::remove_dir_all(&r);
}

// The check's time on this machine for a font of 20,000 glyphs (the Scribe's is in docs/TESTING.md's device
// checks): it ends, within the budget a font has.
fn twenty_thousand_glyphs_are_checked() {
    clean_env();
    let font = ttf(&test_font_many(20000));
    let t = std::time::Instant::now();
    let (f, ..) = passed(check_font(&font, 44));
    eprintln!("    20,095 glyphs ({} KB) checked in {:?}", font.len() / 1024, t.elapsed());
    check!(f.mapped == 20095 && f.skip.is_empty());
    check!(t.elapsed().as_secs() < 120);
}

// One font a job: the job ends after each font that needs a verdict with 5 ("more to do"), so that an export or
// the next slice of a PDF import is not kept waiting for every font of the folder. The last says 0.
fn one_font_a_job() {
    clean_env();
    let r = root("onejob");
    for n in ["A.ttf", "B.ttf", "C.ttf"] {
        put(&r, n, &ttf(&test_font("Regular", 400)));
        // (three files of different bytes: another time each, so the keys differ)
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(100 + n.as_bytes()[0] as u64);
        std::fs::File::options().write(true).open(format!("{r}/fonts/{n}")).unwrap().set_modified(t).unwrap();
    }
    let verdicts = || std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".font")).count();
    check!(bg_font_check("", 300.0) == 5 && verdicts() == 1);
    check!(bg_font_check("next", 300.0) == 5 && verdicts() == 2);
    // the last job's summary is the folder's, never just the one font that job checked ("1 passed")
    let (rc, text) = with_log(&format!("{r}/last.log"), || bg_font_check("next", 300.0));
    check!(rc == 0 && verdicts() == 3);
    check!(text.contains("fonts: in the folder, 3 passed, 0 refused, 0 the worker died on, 0 left for later, 0 too big or empty (3 font file(s) in all)"));
    let (rc, text) = with_log(&format!("{r}/again.log"), || bg_font_check("next", 300.0));
    check!(rc == 0 && verdicts() == 3);
    check!(text.contains("fonts: in the folder, 3 passed,"));
}

/// Runs `f` with stderr (the log) going to a file; the log's text.
fn with_log<R>(path: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    let r = f();
    let _ = std::io::stderr().flush();
    // SAFETY: dup2 and close on the descriptor saved above.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    let text = std::fs::read_to_string(path).unwrap_or_default();
    eprint!("{text}");
    (r, text)
}

fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}
fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// `font` with its cmap replaced by one format 12 subtable of these groups, in this order.
fn with_cmap12(font: &[u8], groups: &[(u32, u32, u32)]) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend(be16(0));
    t.extend(be16(1));
    t.extend(be16(3));
    t.extend(be16(10));
    t.extend(be32(12));
    t.extend(be16(12));
    t.extend(be16(0));
    t.extend(be32(16 + 12 * groups.len() as u32));
    t.extend(be32(0));
    t.extend(be32(groups.len() as u32));
    for &(a, b, g) in groups {
        t.extend(be32(a));
        t.extend(be32(b));
        t.extend(be32(g));
    }
    let mut v = font.to_vec();
    while v.len() % 4 != 0 {
        v.push(0);
    }
    let at = v.len() as u32;
    v.extend(&t);
    let n = u16::from_be_bytes([v[4], v[5]]) as usize;
    for i in 0..n {
        let r = 12 + i * 16;
        if &v[r..r + 4] == b"cmap" {
            v[r + 8..r + 12].copy_from_slice(&be32(at));
            v[r + 12..r + 16].copy_from_slice(&be32(t.len() as u32));
        }
    }
    v
}

// Invariant 4.2.1: the check's walk covers every glyph the UI will ask for. The UI asks Charmap::map (a search
// over the cmap's groups); the walk used Charmap::mappings (the groups in file order), and on groups that
// overlap the two name different glyphs: the UI could be the first to outline a glyph. Now the walk asks map too.
fn the_walk_and_the_ui_read_the_cmap_the_same_way() {
    clean_env();
    let f = wild_glyphs(Wild::Dense);
    let dense = glyph_index(&f, '\u{b6}');
    let plain = ttf(&f);
    let crafted = with_cmap12(&plain, &[(0x20, 0x21, 1), (0x21, 0x21, dense), (0x22, 0x7E, 3)]);
    // (the font is refused now, its ASCII '!' being a glyph over the caps; a pass must have walked it)
    let FontOut::Passed { facts, .. } = check_font(&crafted, 44) else { return };
    let mut face = SafeFace::new(std::rc::Rc::new(crafted.clone()), std::rc::Rc::new(facts.clone()), false).expect("the face is made");
    let got = face.glyph(0x21);
    // what the UI asks for is either a glyph the walk visited and passed, or not drawn at all
    check!(got.is_none_or(|(g, _)| g != dense || facts.skip.contains(&(g as u16))));
}

// The glyph limit, in the tests below: 100 s of CPU for one glyph's draw. A test glyph's real draw is far under
// a millisecond, so what is "slow" is decided by the test charge alone (a pretend cost added inside the face's
// own outline function), with tens of seconds of margin: no test here compares a real clock with a real limit.
const LIMIT_US: u64 = 100_000_000;

// The walk's arithmetic, in this process: a draw is timed through the face the UI draws by, in microseconds
// against the limit; a draw over it is measured again and the smallest counts, unless it is far over; the
// bold through the weight axis is timed as the regular is.
fn the_walk_times_the_faces_own_draw() {
    clean_env();
    let f = test_font_latin1("Regular", 400);
    let font = ttf(&f);
    let g = glyph_index(&f, '\u{a4}');
    let charge = |kind, us| Some(TestSlow { kind, glyph: g, us });
    // nothing charged: nothing is slow, and nothing fails
    let (broken, slow, _) = glyphs_slow_in_process(&font, LIMIT_US, None).unwrap();
    check!(broken.is_empty() && slow.is_empty());
    // half the limit on one glyph: not slow, and the dearest draw is that one
    let (broken, slow, dearest) = glyphs_slow_in_process(&font, LIMIT_US, charge(TestSlowKind::Always, LIMIT_US / 2)).unwrap();
    check!(broken.is_empty() && slow.is_empty());
    check!(dearest >= LIMIT_US / 2 && dearest < LIMIT_US);
    // one microsecond over: that glyph is slow, and no other
    let (broken, slow, dearest) = glyphs_slow_in_process(&font, LIMIT_US, charge(TestSlowKind::Always, LIMIT_US + 1)).unwrap();
    check!(broken.is_empty() && slow == vec![g]);
    check!(dearest > LIMIT_US && dearest < 2 * LIMIT_US);
    // over once: measured again, and the smallest counts
    let (_, slow, dearest) = glyphs_slow_in_process(&font, LIMIT_US, charge(TestSlowKind::Once, LIMIT_US + 1)).unwrap();
    check!(slow.is_empty() && dearest < LIMIT_US);
    let (_, slow, _) = glyphs_slow_in_process(&font, LIMIT_US, charge(TestSlowKind::Once, 2 * LIMIT_US)).unwrap();
    check!(slow.is_empty());
    // far over once (more than four times the limit): not measured again
    let (_, slow, dearest) = glyphs_slow_in_process(&font, LIMIT_US, charge(TestSlowKind::Once, 4 * LIMIT_US + 1)).unwrap();
    check!(slow == vec![g] && dearest > 4 * LIMIT_US);
    // every glyph from one on
    let (_, slow, _) = glyphs_slow_in_process(&font, LIMIT_US, Some(TestSlow { kind: TestSlowKind::From, glyph: g, us: LIMIT_US + 1 })).unwrap();
    check!(slow.first() == Some(&g) && slow.len() == f.glyphs.len() - g as usize && slow.windows(2).all(|w| w[0] + 1 == w[1]));
    // a font with a bold location: the bold face's draw is timed too; a font without one has no such face
    let v = test_font_variable();
    let a = glyph_index(&v, 'A');
    let bold = Some(TestSlow { kind: TestSlowKind::Bold, glyph: a, us: LIMIT_US + 1 });
    let (broken, slow, _) = glyphs_slow_in_process(&ttf(&v), LIMIT_US, bold).unwrap();
    check!(broken.is_empty() && slow == vec![a]);
    let plain = test_font("Regular", 400);
    let (_, slow, dearest) = glyphs_slow_in_process(&ttf(&plain), LIMIT_US, bold).unwrap();
    check!(slow.is_empty() && dearest < LIMIT_US);
    // a glyph that does not draw is broken, not slow, whatever it is charged; and with no limit the walk is
    // the one the corpus is read by
    let far = wild_glyphs(Wild::Far);
    let sect = glyph_index(&far, '\u{a7}');
    let (broken, slow, dearest) = glyphs_slow_in_process(&ttf(&far), LIMIT_US, Some(TestSlow { kind: TestSlowKind::Always, glyph: sect, us: 4 * LIMIT_US + 1 })).unwrap();
    check!(broken == vec![sect] && slow.is_empty() && dearest < LIMIT_US);
    check!(glyphs_failing_in_process(&ttf(&far)).is_ok_and(|b| b == vec![sect]));
}

fn slow_charge(kind: &str, glyph: u32, us: u64) {
    std::env::set_var("GRIMOIRE_FONT_TEST_SLOW", format!("{kind}:{glyph}:{us}"));
}

// Through a worker: a glyph too slow to draw comes from Terminess, as one that does not draw; a letter or a
// digit that is, or more slow glyphs than a font may skip, is a font that takes too long, never one the worker
// died on; the count is the writer's, across slices; the figures reach the writer from every slice.
fn a_glyph_too_slow_to_draw() {
    clean_env();
    std::env::set_var("GRIMOIRE_FONT_GLYPH_US", LIMIT_US.to_string());
    let over = LIMIT_US + 1;
    // one glyph of 191 that is no letter: skipped, and the font passes
    let f = test_font_latin1("Regular", 400);
    let g = glyph_index(&f, '\u{a4}');
    slow_charge("slow", g, over);
    let (facts, _, slow, dearest, _) = passed_timed(check_font(&ttf(&f), 44));
    check!(facts.mapped == 191 && facts.skip == vec![g as u16] && slow == 1 && dearest >= over);
    // (the font's face in the UI then takes that glyph from Terminess)
    let mut face = SafeFace::new(std::rc::Rc::new(ttf(&f)), std::rc::Rc::new(facts), false).expect("the face is made");
    check!(face.glyph(0xa4).is_none() && face.glyph(0xa5).is_some());
    // a letter: the font takes too long
    slow_charge("slow", glyph_index(&f, 'A'), over);
    check!(matches!(check_font(&ttf(&f), 44), FontOut::SlowGlyphs { letter: true, slowest_us, .. } if slowest_us >= over));
    slow_charge("bold", glyph_index(&f, '7'), over);
    check!(matches!(check_font(&ttf(&test_font_variable()), 44), FontOut::SlowGlyphs { letter: true, .. }));
    // one glyph of 95: more than one in a hundred, so too slow (and not "unreadable": nothing is broken)
    let small = test_font("Regular", 400);
    slow_charge("slow", glyph_index(&small, '~'), over);
    check!(matches!(check_font(&ttf(&small), 44), FontOut::SlowGlyphs { letter: false, slow: 1, slowest_us, .. } if slowest_us >= over));
    // too many glyphs that do not draw stay "unreadable" with a slow one among the rest (the second broken
    // glyph ends the check, before the slow one is reached)
    let wild = wild_glyphs(Wild::TooMany);
    slow_charge("slow", glyph_index(&wild, '\u{a4}'), over);
    check!(refused(check_font(&ttf(&wild), 44)) == Refuse::Unreadable);
    // across slices: every glyph from index 500 on is slow; one in a hundred of 4,095 is 40, so the 41st ends
    // the check, as too slow: never as a worker that broke the protocol, never as unreadable
    let many = ttf(&test_font_many(4000));
    slow_charge("from", 500, over);
    std::env::set_var("GRIMOIRE_FONT_SLICE_MS", "2");
    check!(matches!(check_font(&many, 44), FontOut::SlowGlyphs { letter: false, slow: 41, slowest_us, .. } if slowest_us >= over));
    std::env::remove_var("GRIMOIRE_FONT_SLICE_MS");
    check!(matches!(check_font(&many, 44), FontOut::SlowGlyphs { letter: false, slow: 41, .. }));
    // one slow glyph in the middle of the font, in one slice and in many: the same facts, skip list and
    // preview, and the figure both ways
    slow_charge("slow", 2000, over);
    let (one, pv1, slow1, dearest1, _) = passed_timed(check_font(&many, 44));
    std::env::set_var("GRIMOIRE_FONT_SLICE_MS", "2");
    let (sliced, pv2, slow2, dearest2, _) = passed_timed(check_font(&many, 44));
    check!(one.skip == vec![2000] && sliced == one && pv2 == pv1 && (slow1, slow2) == (1, 1));
    check!(dearest1 >= over && dearest2 >= over);
    // a draw that is dear and not slow, in a slice that is not the last: its figure comes with that slice's
    // end; and the letters' and digits' draws add up across the slices
    slow_charge("slow", 2000, LIMIT_US / 2);
    let (f2, _, slow, dearest, _) = passed_timed(check_font(&many, 44));
    check!(f2.skip.is_empty() && slow == 0 && dearest >= LIMIT_US / 2 && dearest < LIMIT_US);
    slow_charge("slow", glyph_index(&test_font_many(4000), 'A'), LIMIT_US / 2);
    let (f3, _, _, dearest, letters) = passed_timed(check_font(&many, 44));
    check!(f3.skip.is_empty() && dearest >= LIMIT_US / 2 && letters >= LIMIT_US / 2 && letters < LIMIT_US);
    std::env::remove_var("GRIMOIRE_FONT_SLICE_MS");
    let (_, _, _, _, letters) = passed_timed(check_font(&many, 44));
    check!(letters >= LIMIT_US / 2 && letters < LIMIT_US);
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");
    // a limit that is no number a record carries: the default is used (the font is checked, and passes)
    for bad in ["0", "junk", "1000000000", "-5", ""] {
        std::env::set_var("GRIMOIRE_FONT_GLYPH_US", bad);
        let (facts, _, slow, _, _) = passed_timed(check_font(&ttf(&small), 44));
        check!(facts.mapped == 95 && facts.skip.is_empty() && slow == 0);
    }
    // (and a charge that does not read is no charge)
    std::env::set_var("GRIMOIRE_FONT_GLYPH_US", LIMIT_US.to_string());
    for bad in ["slow:40", "quick:40:200000000", "slow:x:200000000", "slow:40:200000000:1"] {
        std::env::set_var("GRIMOIRE_FONT_TEST_SLOW", bad);
        check!(passed(check_font(&ttf(&small), 44)).0.skip.is_empty());
    }
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");
    std::env::remove_var("GRIMOIRE_FONT_GLYPH_US");
}

/// The files of a data root's fonts cache with this extension.
fn with_ext(r: &str, ext: &str) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == ext)).collect()
}

/// Gives a file the time `secs` before now (or after it, for a negative number). The tests of the second try
/// move a mark's time and never wait: a mark set back only gets older while a test runs, and one set a day
/// ahead stays ahead.
fn set_time(file: &std::path::Path, secs: i64) {
    let now = std::time::SystemTime::now();
    let d = std::time::Duration::from_secs(secs.unsigned_abs());
    let t = if secs >= 0 { now - d } else { now + d };
    std::fs::File::options().write(true).open(file).unwrap().set_modified(t).unwrap();
}

/// Two minutes back: the minute a font waits for its second try is over.
fn set_back(mark: &std::path::Path) {
    set_time(mark, 120);
}

// In the folder: a font whose letters take too long to draw is not judged on one measure. The first check
// leaves a mark and no verdict; a minute later it is checked again, and only then is it "Took too long to
// read.". A second check that passes stands, and the mark goes.
fn slow_glyphs_through_the_job() {
    clean_env();
    std::env::set_var("GRIMOIRE_FONT_GLYPH_US", LIMIT_US.to_string());
    let f = test_font("Regular", 400);
    let r = root("slowglyphs");
    put(&r, "Slow.ttf", &ttf(&f));
    slow_charge("slow", glyph_index(&f, 'A'), LIMIT_US + 1);
    let (rc, text) = with_log(&format!("{r}/first.log"), job);
    check!(rc == 3);
    check!(text.contains("fonts: file 1 of 1: glyphs take too long to draw (a letter or digit, slowest ") && text.contains(" us): left for a second try"));
    check!(with_ext(&r, "slow").len() == 1 && std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().count() == 1);
    // (no worker can be had from here to the second check: whatever the clock says, nothing below checks the
    // font before its mark is set back)
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(state_of(&listed(), "Slow.ttf") == "checking");
    // A mark dated ahead of the clock (the clock was set back since): the font waits, no worker is asked for
    // (the job would say 4), and the mark is given the time of now, so the wait is a minute from here and no
    // longer.
    let mark = with_ext(&r, "slow")[0].clone();
    let text = std::fs::read(&mark).unwrap();
    set_time(&mark, -86_400);
    check!(job() == 3);
    check!(std::fs::metadata(&mark).and_then(|m| m.modified()).is_ok_and(|t| t <= std::time::SystemTime::now()));
    check!(std::fs::read(&mark).is_ok_and(|b| b == text));
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    check!(with_ext(&r, "slow").len() == 1 && std::fs::read_dir(format!("{r}/fonts/.cache")).unwrap().count() == 1);
    // the minute over: the second check agrees
    set_back(&with_ext(&r, "slow")[0]);
    let (rc, text) = with_log(&format!("{r}/second.log"), job);
    check!(rc == 0);
    check!(text.contains("fonts: file 1 of 1: glyphs take too long to draw again (a letter or digit, slowest "));
    check!(state_of(&listed(), "Slow.ttf") == "slow");
    check!(with_ext(&r, "slow").is_empty() && with_ext(&r, "bad").is_empty() && with_ext(&r, "pgm").is_empty());
    check!(with_ext(&r, "no").len() == 1 && with_ext(&r, "font").len() == 1);
    check!(std::fs::read_to_string(&with_ext(&r, "font")[0]).is_ok_and(|t| t.contains("\nwhy=slow\n")));
    check!(userfonts_try_load("Slow.ttf", "") == Err(LoadErr::Refused));
    // (and it is not checked again)
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(job() == 0);
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    // Another root: the first check says slow, the second (the charge gone) passes. The pass stands.
    let r = root("slowonce");
    put(&r, "Spike.ttf", &ttf(&f));
    check!(job() == 3);
    check!(with_ext(&r, "slow").len() == 1 && with_ext(&r, "font").is_empty());
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");
    set_back(&with_ext(&r, "slow")[0]);
    let (rc, text) = with_log(&format!("{r}/second.log"), job);
    check!(rc == 0);
    check!(text.contains("fonts: file 1 of 1 passed: 16 KB, 95 glyphs, 0 from Terminess (0 too slow to draw), slowest glyph ") && text.contains(" us, letters and digits "));
    check!(state_of(&listed(), "Spike.ttf") == "ready");
    check!(userfonts_try_load("Spike.ttf", "") == Ok("overprint"));
    check!(with_ext(&r, "slow").is_empty() && with_ext(&r, "no").is_empty() && with_ext(&r, "bad").is_empty());
    // A font that passes with one glyph too slow to draw says so in its line.
    let r = root("slowone");
    let wide = test_font_latin1("Regular", 400);
    put(&r, "Wide.ttf", &ttf(&wide));
    slow_charge("slow", glyph_index(&wide, '\u{a4}'), LIMIT_US + 1);
    let (rc, text) = with_log(&format!("{r}/first.log"), job);
    check!(rc == 0);
    check!(text.contains(" 191 glyphs, 1 from Terminess (1 too slow to draw), slowest glyph 1000000"));
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");
    std::env::remove_var("GRIMOIRE_FONT_GLYPH_US");
}

/// The key of the one font file in the folder, from the listing (the check a scan queues finds no worker and
/// leaves nothing).
fn only_key() -> u64 {
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let l = userfonts_scan(300.0);
    wait_job();
    std::env::set_var("GRIMOIRE_WORK_EXE", bin());
    assert_eq!(l.len(), 1);
    l[0].key
}

/// What a version that wrote verdicts numbered 1 can leave in the cache for a key: an empty `.slow` mark, three
/// days old (it kept the mark of a first timeout beside the verdict of a second check that passed), its verdict
/// and a preview. Returns the mark's path, and the verdict's with its text.
fn leftovers_of_verdict_1(r: &str, key: u64, size: usize) -> (std::path::PathBuf, std::path::PathBuf, String) {
    let cache = format!("{r}/fonts/.cache");
    std::fs::create_dir_all(&cache).unwrap();
    let mark = std::path::PathBuf::from(format!("{cache}/{key:016x}.slow"));
    std::fs::write(&mark, b"").unwrap();
    set_time(&mark, 3 * 86_400);
    let text = format!("grimoire-font 1\nhash=0123456789abcdef\nsize={size}\nok=1\nupem=1000\nascent=800\ndescent=200\nscale=1000\nweight=400\nitalic=0\nmapped=95\ncover=0 0 0 0 0 0\nlacks=0\nfamily=Grimoire Test\nstyle=Regular\n");
    let font = std::path::PathBuf::from(format!("{cache}/{key:016x}.font"));
    std::fs::write(&font, &text).unwrap();
    std::fs::write(format!("{cache}/{key:016x}.pgm"), b"P5\n8 8\n255\n").unwrap();
    (mark, font, text)
}

// The mark of a first try counts only as what it is: this check's (it holds the verdict's number), of these
// very bytes (their SHA-256), and of the kind of result the next check comes to. An empty mark that a version
// with verdicts numbered 1 left in the cache is no first try: the one check after the update that finds a
// letter too slow to draw, or runs out of time, leaves the font for its second try. A check out of time and
// a check that found slow glyphs do not make a verdict together, and a mark of other bytes is no mark. No
// step waits on a clock: a mark is made old with set_modified, and the timeouts are the test watchdog's.
fn a_first_trys_mark_counts_only_for_its_own_check() {
    use grimoire::sha256::{hex32, sha256};
    clean_env();
    std::env::set_var("GRIMOIRE_FONT_GLYPH_US", LIMIT_US.to_string());
    let f = test_font("Regular", 400);
    let bytes = ttf(&f);
    let a = glyph_index(&f, 'A');
    let digest = hex32(&sha256(&bytes));
    let mark_says = |r: &str, kinds: &str| {
        let m = with_ext(r, "slow");
        m.len() == 1 && std::fs::read(&m[0]).is_ok_and(|b| b == format!("grimoire-font 2 slow {kinds} {digest}\n").as_bytes())
    };
    let verdict = |r: &str| with_ext(r, "font").first().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let hang = |on: bool| {
        if on {
            std::env::set_var("GRIMOIRE_FONT_TEST_HANG", "40");
            std::env::set_var("GRIMOIRE_FONT_WATCHDOG_MS", "700");
        } else {
            std::env::remove_var("GRIMOIRE_FONT_TEST_HANG");
            std::env::remove_var("GRIMOIRE_FONT_WATCHDOG_MS");
        }
    };

    // The leftovers, then one check that finds a letter too slow to draw: a first try, no verdict.
    let r = root("left-glyphs");
    put(&r, "Left.ttf", &bytes);
    let (mark, old_verdict, old_text) = leftovers_of_verdict_1(&r, only_key(), bytes.len());
    slow_charge("slow", a, LIMIT_US + 1);
    let (rc, text) = with_log(&format!("{r}/first.log"), job);
    check!(rc == 3);
    check!(text.contains(" us): left for a second try") && !text.contains("again"));
    check!(std::fs::read_to_string(&old_verdict).is_ok_and(|t| t == old_text) && with_ext(&r, "no").is_empty() && with_ext(&r, "bad").is_empty());
    check!(mark_says(&r, "glyphs"));
    // (a minute later the second check agrees: the verdict, and the mark goes)
    set_back(&mark);
    let (rc, text) = with_log(&format!("{r}/second.log"), job);
    check!(rc == 0 && text.contains("glyphs take too long to draw again"));
    check!(verdict(&r).starts_with("grimoire-font 2\n") && verdict(&r).contains("\nwhy=slow\n") && with_ext(&r, "slow").is_empty());
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");

    // The leftovers, then one check that runs out of time: a first try too. (Under the test watchdog the wait
    // for the second try is none.)
    let r = root("left-time");
    put(&r, "Left.ttf", &bytes);
    let (_, old_verdict, old_text) = leftovers_of_verdict_1(&r, only_key(), bytes.len());
    hang(true);
    let (rc, text) = with_log(&format!("{r}/first.log"), job);
    check!(rc == 3);
    check!(text.contains(" ms): left for a second try") && !text.contains("again"));
    check!(std::fs::read_to_string(&old_verdict).is_ok_and(|t| t == old_text) && with_ext(&r, "no").is_empty());
    check!(mark_says(&r, "time"));
    let (rc, text) = with_log(&format!("{r}/second.log"), job);
    check!(rc == 0 && text.contains("took too long to read again"));
    check!(verdict(&r).contains("\nwhy=slow\n") && with_ext(&r, "slow").is_empty());
    hang(false);

    // One check out of time, then one that finds a letter too slow: one reading of each kind is no verdict.
    // The second reading of a kind is.
    let r = root("mixed-time-first");
    put(&r, "Mixed.ttf", &bytes);
    hang(true);
    check!(job() == 3 && mark_says(&r, "time"));
    hang(false);
    slow_charge("slow", a, LIMIT_US + 1);
    set_back(&with_ext(&r, "slow")[0]);
    let (rc, text) = with_log(&format!("{r}/second.log"), job);
    check!(rc == 3);
    check!(text.contains("glyphs take too long to draw (a letter or digit, slowest ") && text.contains(" us): left for a second try") && !text.contains("again"));
    check!(mark_says(&r, "time,glyphs") && with_ext(&r, "font").is_empty() && with_ext(&r, "no").is_empty());
    set_back(&with_ext(&r, "slow")[0]);
    let (rc, text) = with_log(&format!("{r}/third.log"), job);
    check!(rc == 0 && text.contains("glyphs take too long to draw again"));
    check!(verdict(&r).contains("\nwhy=slow\n") && with_ext(&r, "slow").is_empty());
    // (and the other way round: slow glyphs first, then one check out of time)
    let r = root("mixed-glyphs-first");
    put(&r, "Mixed.ttf", &bytes);
    check!(job() == 3 && mark_says(&r, "glyphs"));
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");
    hang(true);
    let (rc, text) = with_log(&format!("{r}/second.log"), job);
    check!(rc == 3);
    check!(text.contains(" ms): left for a second try") && !text.contains("again"));
    check!(mark_says(&r, "time,glyphs") && with_ext(&r, "font").is_empty());
    let (rc, text) = with_log(&format!("{r}/third.log"), job);
    check!(rc == 0 && text.contains("took too long to read again"));
    check!(verdict(&r).contains("\nwhy=slow\n") && with_ext(&r, "slow").is_empty());
    hang(false);

    // A mark of other bytes (another file had this name, size and time), a mark of another verdict number, and
    // a mark with a word this version does not write: each is no first try, and goes.
    let other = hex32(&sha256(b"another file"));
    for (i, text) in [format!("grimoire-font 2 slow glyphs {other}\n"), format!("grimoire-font 3 slow glyphs {digest}\n"), format!("grimoire-font 2 slow often {digest}\n"), format!("grimoire-font 2 slow glyphs {digest}")].iter().enumerate() {
        let r = root(&format!("not-its-mark-{i}"));
        put(&r, "Other.ttf", &bytes);
        let key = only_key();
        let mark = std::path::PathBuf::from(format!("{r}/fonts/.cache/{key:016x}.slow"));
        std::fs::write(&mark, text).unwrap();
        set_back(&mark);
        slow_charge("slow", a, LIMIT_US + 1);
        let (rc, log) = with_log(&format!("{r}/first.log"), job);
        check!(rc == 3 && log.contains(" us): left for a second try") && !log.contains("again"));
        check!(mark_says(&r, "glyphs") && with_ext(&r, "font").is_empty() && with_ext(&r, "no").is_empty());
    }
    std::env::remove_var("GRIMOIRE_FONT_TEST_SLOW");
    std::env::remove_var("GRIMOIRE_FONT_GLYPH_US");
}

// The command a worker takes is this build's alone: one of the grammar before (six fields) is refused, and so
// is a glyph limit of nothing.
fn the_worker_refuses_a_command_of_another_grammar() {
    use grimoire::sandbox::{mono_ms, WorkKind, Worker};
    clean_env();
    for (command, want) in [("F 10 0 44 1000 0\n", "E protocol"), ("F 10 0 44 1000 0 0\n", "E protocol"), ("F 10 0 44 1000 0 x\n", "E protocol"), ("F 10 0 44 1000 0 5000 1\n", "E protocol")] {
        let mut w = Worker::spawn(WorkKind::Font, None, &limits(10)).expect("a worker starts");
        let deadline = mono_ms() + 15_000;
        let hello = w.read_line(512, deadline).unwrap_or_default();
        check!(hello.starts_with(b"B "));
        let sandbox = w.read_line(512, deadline).unwrap_or_default();
        check!(sandbox == b"S 0" || sandbox == b"S 1");
        check!(w.send(command));
        let answer = w.read_line(512, deadline).unwrap_or_default();
        check!(answer == want.as_bytes());
        if answer != want.as_bytes() {
            eprintln!("    {command:?}: {:?}", String::from_utf8_lossy(&answer));
        }
        w.kill_and_reap();
    }
    // (this build's own command is taken: ten bytes that are no font get the font's answer, not the grammar's)
    let mut w = Worker::spawn(WorkKind::Font, None, &limits(10)).expect("a worker starts");
    let deadline = mono_ms() + 15_000;
    let _ = (w.read_line(512, deadline), w.read_line(512, deadline));
    check!(w.send("F 10 0 44 1000 0 5000\n") && w.send("0123456789"));
    check!(w.read_line(512, deadline).is_ok_and(|l| l == b"E bad 0"));
    w.kill_and_reap();
}

#[test]
fn test_fontcheck() {
    run!(good_fonts_pass);
    run!(the_corpus_gets_verdicts_never_a_crash);
    run!(a_killed_worker_leaves_bad_and_the_rest_go_on);
    run!(slow_fonts_and_slices);
    run!(the_folder_is_listed_by_the_walk);
    run!(a_file_still_being_copied_waits);
    run!(a_replaced_file_is_checked_again_and_bold_is_found);
    run!(the_loaded_fonts_are_capped);
    run!(twenty_thousand_glyphs_are_checked);
    run!(one_font_a_job);
    run!(the_walk_and_the_ui_read_the_cmap_the_same_way);
    run!(the_walk_times_the_faces_own_draw);
    run!(a_glyph_too_slow_to_draw);
    run!(slow_glyphs_through_the_job);
    run!(a_first_trys_mark_counts_only_for_its_own_check);
    run!(the_worker_refuses_a_command_of_another_grammar);
    run!(a_fonts_sample_is_drawn_by_the_worker);
    // (last: it leaves the session without workers, as a failed step does)
    run!(a_failed_sandbox_step_checks_nothing);
    done();
}
