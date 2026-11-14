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

const HOOKS: [&str; 11] = [
    "GRIMOIRE_STATE",
    "GRIMOIRE_STATE_FREE_BYTES",
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
        FontOut::Passed { facts, preview, w, h } => (facts, preview, w, h),
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
    // (last: it leaves the session without workers, as a failed step does)
    run!(a_failed_sandbox_step_checks_nothing);
    done();
}
