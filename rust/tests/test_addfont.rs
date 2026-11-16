// test_addfont.rs (2.2, issue #161): a font added from a file, by the Font sheet's Add a font... or by Add to fonts
// on a font's card in Files. The rule these cases hold: the copy gives a font nothing. What lands in grimoire/fonts
// is a file there, as one put there over USB, and Grimoire reads it as a font only after the sandboxed worker has
// passed these very bytes. Until then it is listed and no more: not loaded, not drawn, no preview of it read.
//
// Every case goes the way the app goes: the font pick's own rows and card on a drawn screen (browser.rs), the real
// copy job forked as the app forks it (bgwork_copy.rs), the real font check with a real worker, a fresh exec of the
// headless runner (userfonts.rs, fontcheck.rs). The fonts are made here (tests/fontfix). One #[test], its cases in
// order: the storage root, the store's root and the environment are the process's.
//
//  - the pick copies a font out of the Kindle's own fonts folder, and only its check makes it one
//  - a font that fails its check after the copy is listed with its reason and never loaded
//  - a copy cut short (a failed write, a temporary file a dead copy left) is no font, and the next copy removes it
//  - the source is the plain file that was listed: no link, and not another file put in its place meanwhile
//  - a file over the font check's limit is refused before the room is looked at; the free-space floor holds
//  - a name taken in grimoire/fonts is never replaced; a name the rules refuse is not copied
//  - the Kindle's own fonts folder is as it was after all of it
//  - the folder is looked at for a copied font once the file is as old as the check asks of any file
#![cfg(feature = "sim")]
mod fontfix;
mod testlib;
use fontfix::*;
use grimoire::bgwork_copy::*;
use grimoire::browser::{Browser, BrowserResult, FilesPrefs, PickSpec};
use grimoire::fontcheck::*;
use grimoire::fsview::*;
use grimoire::prelude::*;
use grimoire::userfonts::*;
use std::os::unix::fs::{symlink, MetadataExt};
use testlib::*;

const BIN: &str = env!("CARGO_BIN_EXE_grimoire-headless");
const NO_WORKER: &str = "/nonexistent/grimoire";
const FULL: &str = "Couldn't copy file. Is the storage full?";

fn sp(s: &str) -> StoragePath {
    StoragePath::new(s.as_bytes()).unwrap_or_else(|| panic!("{s}"))
}

fn good() -> Vec<u8> {
    ttf(&test_font("Regular", 400))
}

/// A storage made afresh: the Kindle's own fonts folder with two good fonts, a folder of documents, and Grimoire's
/// folder (its store set up there) with an empty fonts folder. The worker is the headless runner, a file is taken
/// as whole at once, and no test seam is left set.
fn tree(name: &str) -> (String, String) {
    let root = format!("{}/addfont-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    let g = format!("{root}/grimoire");
    for d in ["fonts", "documents/Type", "grimoire/fonts"] {
        std::fs::create_dir_all(format!("{root}/{d}")).unwrap();
    }
    std::fs::write(format!("{root}/fonts/Serif.ttf"), good()).unwrap();
    std::fs::write(format!("{root}/fonts/Serif-Bold.ttf"), ttf(&test_font("Bold", 700))).unwrap();
    for h in ["GRIMOIRE_STATE", "GRIMOIRE_BG_DELAY_MS", "GRIMOIRE_FONT_WATCHDOG_MS", "GRIMOIRE_SANDBOX"] {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_STORAGE", &root);
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    copy_seams(None, None);
    store_init(&g);
    (root, g)
}

fn names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// What a folder holds, to compare before and after: each name with its bytes and its time.
fn snapshot(dir: &str) -> Vec<(String, Vec<u8>, i64)> {
    names(dir).into_iter().map(|n| {
        let p = format!("{dir}/{n}");
        (n, std::fs::read(&p).unwrap_or_default(), std::fs::metadata(&p).map(|m| m.mtime()).unwrap_or(0))
    }).collect()
}

/// The check's results kept for the fonts folder: its verdicts, and the previews beside them.
fn kept(g: &str, ext: &str) -> usize {
    names(&format!("{g}/fonts/.cache")).iter().filter(|n| n.ends_with(ext)).count()
}

/// Waits for the background children to end, reaping each (the next font's check queued as the app queues it):
/// the last result a job the user waits on gave (1 done, -1 failed), 0 for none.
fn settle() -> i32 {
    let mut r = 0;
    let t0 = grimoire::util::real_now_ms();
    while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 60_000 {
        let mut kind = -1;
        let got = bg_poll_kind(Some(&mut kind));
        if got != 0 {
            r = got;
        }
        if kind == BgKind::FontCheck as i32 && bg_last_exit() == 5 {
            bg_queue(BgKind::FontCheck, Some(""), Some("next"), 300.0, false);
        }
        sleep_ms(5);
    }
    check!(!bg_busy());
    r
}

/// The font check as the background child runs it, in this process, to its end.
fn job() -> i32 {
    loop {
        let rc = bg_font_check("", 300.0);
        if rc != 5 {
            return rc;
        }
    }
}

/// The fonts folder as the Font sheet lists it, once the check a look queues has ended.
fn listed() -> Vec<UserFont> {
    let mut l = userfonts_scan(300.0);
    settle();
    userfonts_refresh(&mut l);
    for f in l.iter_mut() {
        userfont_resolve(f);
    }
    settle();
    l
}

fn state_of(l: &[UserFont], rel: &str) -> &'static str {
    l.iter().find(|f| f.rel == rel).map_or("not listed", |f| f.state_name())
}

/// A screen to draw the picker on (1860 x 2480, upright).
fn screen() -> App {
    check!(font_init() == 0);
    ui_init(300.0);
    i18n_set("en");
    let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
    check!(canvas_init(&mut a.screen, a.w, a.h) == 0);
    let (mut w, mut h) = (a.w, a.h);
    grimoire::sim_sdl::display_set_orientation(0, &mut w, &mut h);
    a
}

fn probe(b: &Browser, k: &str) -> String {
    let mut o = String::new();
    check!(b.probe(2480, k, &mut o));
    o
}

fn tap(b: &mut Browser, a: &mut App, name: &str) -> BrowserResult {
    let (x, y) = b.point(name).unwrap_or_else(|| panic!("no {name} to tap"));
    let ev = InEvent { x, y, t_ms: now_ms(), ..Default::default() };
    b.pointer(a, 0, &ev);
    b.pointer(a, 2, &ev)
}

/// The font pick as the Font sheet's Add a font... opens it, in `folder`, drawn.
fn pick_in(a: &mut App, folder: &str) -> Browser {
    let mut b = Browser::default();
    check!(b.open(PickSpec::font(), FilesPrefs { card: true, ..Default::default() }));
    check!(b.go(sp(folder)));
    b.draw(a);
    b
}

/// A tap on the font `name` of the folder the pick shows, then on its card's Add to fonts: what the pick said to
/// its host, and the toast up after it ("" for none).
fn add(b: &mut Browser, a: &mut App, name: &str) -> (BrowserResult, String) {
    a.toast.clear();
    tap(b, a, &format!("item.{name}"));
    check!(probe(b, "card") == "font");
    check!(probe(b, "card.rows") == "Add to fonts|Cancel");
    let r = tap(b, a, "card.add");
    (r, a.toast.as_str().to_string())
}

// The pick, from the Kindle's own fonts folder: its rows are names, sizes and dates; the card's Add to fonts asks
// the copy job, which puts the bytes in grimoire/fonts. With no worker to be had, nothing more happens to the file,
// however often the folder is looked at: it is listed as being checked, it cannot be used, no preview of it is read,
// and a load of it is refused before a byte of it is parsed. Once the worker has passed it, it loads. And the pass
// is of those bytes: other bytes under the same name, size and time do not load.
fn the_pick_copies_and_only_the_check_makes_a_font() {
    let (root, g) = tree("pick");
    let kindle = snapshot(&format!("{root}/fonts"));
    let mut a = screen();
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    let mut b = pick_in(&mut a, "fonts");
    check!(probe(&b, "mode") == "pick" && probe(&b, "crumbs") == "Files > Kindle fonts");
    check!(probe(&b, "order") == "Serif-Bold.ttf|Serif.ttf");
    let (said, toast) = add(&mut b, &mut a, "Serif.ttf");
    check!(said == BrowserResult::Copying(CopyTo::Fonts) && toast.is_empty());
    check!(settle() == 1);
    check!(names(&format!("{g}/fonts")) == ["Serif.ttf"]);
    check!(std::fs::read(format!("{g}/fonts/Serif.ttf")).unwrap() == good());
    // no check has passed it: listed, and nothing else
    for _ in 0..3 {
        let l = listed();
        check!(l.len() == 1 && state_of(&l, "Serif.ttf") == "checking");
        let mut f = l[0].clone();
        check!(!f.usable() && userfont_preview(&mut f).is_none());
        check!(userfonts_try_load("Serif.ttf", "") == Err(LoadErr::Checking));
        settle();
        check!(!userfonts_in_use("Serif.ttf"));
    }
    check!(kept(&g, ".font") == 0 && kept(&g, ".pgm") == 0);
    // the worker passes it: a font from now on
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    check!(job() == 0);
    check!(kept(&g, ".font") == 1 && kept(&g, ".pgm") == 1);
    let l = listed();
    check!(state_of(&l, "Serif.ttf") == "ready" && l[0].usable());
    check!(userfonts_try_load("Serif.ttf", "").is_ok());
    // what passed is those bytes: others under the same name, size and time are not loaded, and are checked anew
    let path = format!("{g}/fonts/Serif.ttf");
    let md = std::fs::metadata(&path).unwrap();
    let mut other = good();
    let (glyf, _) = table(&other, b"glyf");
    other[glyf + 300] ^= 0x01;
    std::fs::write(&path, &other).unwrap();
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(md.modified().unwrap()).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", NO_WORKER);
    check!(userfonts_try_load("Serif.ttf", "") == Err(LoadErr::Checking));
    settle();
    std::env::set_var("GRIMOIRE_WORK_EXE", BIN);
    // the Kindle's folder: as it was
    check!(snapshot(&format!("{root}/fonts")) == kindle);
    b.close();
    canvas_free(&mut a.screen);
}

// A font that fails its check after the copy: a file cut short, a symbol font, and a file that is no font at all
// under a font's name are copied as any file is, and the check says of each what it is. None is ever loaded; each
// stays in the folder, listed with its reason.
fn a_font_that_fails_its_check_is_listed_and_never_loaded() {
    let (root, g) = tree("refused");
    let cut = good()[..700].to_vec();
    for (n, bytes) in [("Cut.ttf", cut), ("Symbols.ttf", ttf(&symbol_cmap())), ("Text.otf", b"only words, no font".to_vec())] {
        std::fs::write(format!("{root}/documents/Type/{n}"), bytes).unwrap();
    }
    let mut a = screen();
    for n in ["Cut.ttf", "Symbols.ttf", "Text.otf"] {
        let mut b = pick_in(&mut a, "documents/Type");
        check!(add(&mut b, &mut a, n).0 == BrowserResult::Copying(CopyTo::Fonts));
        check!(settle() == 1);
        b.close();
    }
    check!(names(&format!("{g}/fonts")) == ["Cut.ttf", "Symbols.ttf", "Text.otf"]);
    check!(job() == 0);
    let l = listed();
    check!(state_of(&l, "Cut.ttf") == "unreadable" && state_of(&l, "Text.otf") == "unreadable" && state_of(&l, "Symbols.ttf") == "nolatin");
    for f in &l {
        let mut f = f.clone();
        check!(!f.usable() && userfont_preview(&mut f).is_none());
        check!(userfonts_try_load(&f.rel, "") == Err(LoadErr::Refused));
    }
    check!(kept(&g, ".pgm") == 0);
    canvas_free(&mut a.screen);
}

// A copy cut short. One whose write fails (a full storage) leaves nothing in the folder. A temporary file a copy
// left when its process died (the power) holds a font's first bytes, or all of them, under the copy's temporary
// name: no listing shows it, the fonts folder's scan does not take it, the check job reads nothing of it and keeps
// no result for it, and the next copy into the folder removes it.
fn a_copy_cut_short_is_no_font() {
    let (root, g) = tree("cut");
    let mut a = screen();
    let fonts = format!("{g}/fonts");
    copy_seams(None, Some(4096));
    let mut b = pick_in(&mut a, "fonts");
    check!(add(&mut b, &mut a, "Serif.ttf").0 == BrowserResult::Copying(CopyTo::Fonts));
    check!(settle() == -1);
    check!(copy_refusal(CopyFail::of_exit(bg_last_exit()), CopyTo::Fonts) == FULL);
    copy_seams(None, None);
    check!(names(&fonts).is_empty());
    // what a dead copy left: a whole font, and half of one (a process id no system gives)
    let whole = format!("{fonts}/.grimoire-copy-{}.part", i32::MAX);
    let half = format!("{fonts}/.grimoire-copy-{}.part", i32::MAX - 1);
    std::fs::write(&whole, good()).unwrap();
    std::fs::write(&half, &good()[..5000]).unwrap();
    check!(listed().is_empty());
    check!(job() == 0);
    check!(kept(&g, ".font") == 0 && kept(&g, ".pgm") == 0 && kept(&g, ".checking") == 0 && kept(&g, ".bad") == 0);
    check!(userfonts_try_load(&format!(".grimoire-copy-{}.part", i32::MAX), "") == Err(LoadErr::Missing));
    // Files does not list it either, with every file shown
    let mut f = Browser::default();
    check!(f.open(PickSpec::place(), FilesPrefs { card: true, sort: 0, show: 1 }));
    check!(f.go(sp("grimoire/fonts")));
    check!(probe(&f, "rows") == "0");
    f.close();
    // the next copy into the folder takes them away
    b.close();
    let mut b = pick_in(&mut a, "fonts");
    check!(add(&mut b, &mut a, "Serif.ttf").0 == BrowserResult::Copying(CopyTo::Fonts));
    check!(settle() == 1);
    // (the check's own folder, made by the job above, and the font: no temporary file)
    check!(names(&fonts) == [".cache", "Serif.ttf"]);
    let _ = root;
    b.close();
    canvas_free(&mut a.screen);
}

// The source is the plain file that was listed. A link is no row of the pick, and a request made for one anyway is
// refused, as is one for a file behind a linked folder; the job refuses both too. A file put in the place of the
// one listed, between the listing and the tap, is not copied: "File changed. Try again.", no job, and the job
// itself refuses a source that is not the file its request names.
fn the_source_is_the_file_that_was_listed() {
    let (root, g) = tree("source");
    let d = format!("{root}/documents/Type");
    std::fs::write(format!("{d}/Sans.ttf"), good()).unwrap();
    symlink(format!("{root}/fonts/Serif.ttf"), format!("{d}/Link.ttf")).unwrap();
    symlink(format!("{root}/fonts"), format!("{root}/documents/Linked")).unwrap();
    let st = FsStorage::open(&root, &g).unwrap();
    let mut a = screen();
    let mut b = pick_in(&mut a, "documents/Type");
    check!(probe(&b, "order") == "Sans.ttf");
    let arg = |s: &FsStat, p: &str| format!("{}.{}.{}.{}:{}", s.dev, s.ino, s.size, s.mtime, p);
    // a link, asked for by hand: its own stat is no plain file's; the folder link is not walked
    let link = st.stat(&sp("documents/Type/Link.ttf"), true).unwrap();
    check!(copy_request(&st, &sp("documents/Type/Link.ttf"), &link, CopyTo::Fonts) == Err(FsWriteErr::Walk(FsErr::NotFile)));
    let real = st.stat(&sp("fonts/Serif.ttf"), true).unwrap();
    check!(copy_request(&st, &sp("documents/Type/Link.ttf"), &real, CopyTo::Fonts) == Err(FsWriteErr::Changed));
    check!(copy_request(&st, &sp("documents/Linked/Serif.ttf"), &real, CopyTo::Fonts) == Err(FsWriteErr::Walk(FsErr::Link)));
    check!(bg_copy_file("fonts", &arg(&link, "documents/Type/Link.ttf")) == -1);
    check!(bg_copy_file("fonts", &arg(&real, "documents/Type/Link.ttf")) == -1);
    check!(bg_copy_file("fonts", &arg(&real, "documents/Linked/Serif.ttf")) == -1);
    check!(!bg_busy() && names(&format!("{g}/fonts")).is_empty());
    // swapped between the listing and the tap: another file of the same size under the name
    tap(&mut b, &mut a, "item.Sans.ttf");
    check!(probe(&b, "card.rows") == "Add to fonts|Cancel");
    let listed = st.stat(&sp("documents/Type/Sans.ttf"), true).unwrap();
    let mut other = good();
    other[400] ^= 0xff;
    std::fs::write(format!("{d}/.swap"), &other).unwrap();
    std::fs::rename(format!("{d}/.swap"), format!("{d}/Sans.ttf")).unwrap();
    a.toast.clear();
    check!(tap(&mut b, &mut a, "card.add") == BrowserResult::None);
    check!(a.toast.as_str() == "File changed. Try again.");
    check!(!bg_busy() && names(&format!("{g}/fonts")).is_empty());
    // and the job, handed the file as it was listed
    check!(bg_copy_file("fonts", &arg(&listed, "documents/Type/Sans.ttf")) == CopyFail::Changed as i32);
    check!(names(&format!("{g}/fonts")).is_empty());
    b.close();
    canvas_free(&mut a.screen);
}

// A file over what the font check takes of one file (24 MB) is refused when it is asked for, with room or without:
// "Too big (over 24 MB).", no job, nothing written. A font that fits is refused when the copy would leave less than
// the floor (100 MB after it), and copied when the floor holds to the byte.
fn too_big_and_no_room() {
    let (root, g) = tree("room");
    let d = format!("{root}/documents/Type");
    std::fs::File::create(format!("{d}/Huge.ttf")).unwrap().set_len(FONT_FILE_MAX as u64 + 1).unwrap();
    std::fs::write(format!("{d}/Sans.ttf"), good()).unwrap();
    check!(CopyTo::Fonts.max_bytes() == FONT_FILE_MAX as u64);
    let size = good().len() as u64;
    let mut a = screen();
    for free in [None, Some(1 << 20)] {
        copy_seams(free, None);
        let mut b = pick_in(&mut a, "documents/Type");
        let (said, toast) = add(&mut b, &mut a, "Huge.ttf");
        check!(said == BrowserResult::None && toast == "Too big (over 24 MB).");
        check!(!bg_busy());
        b.close();
    }
    copy_seams(Some(FS_FREE_FLOOR + size - 1), None);
    let mut b = pick_in(&mut a, "documents/Type");
    let (said, toast) = add(&mut b, &mut a, "Sans.ttf");
    check!(said == BrowserResult::None && toast == FULL);
    check!(!bg_busy() && names(&format!("{g}/fonts")).is_empty());
    copy_seams(Some(FS_FREE_FLOOR + size), None);
    let (said, toast) = add(&mut b, &mut a, "Sans.ttf");
    check!(said == BrowserResult::Copying(CopyTo::Fonts) && toast.is_empty());
    check!(settle() == 1);
    copy_seams(None, None);
    check!(names(&format!("{g}/fonts")) == ["Sans.ttf"]);
    b.close();
    canvas_free(&mut a.screen);
}

// A name taken in grimoire/fonts is never replaced: the copy is " 2" (then " 3"), and the file that had the name
// keeps its bytes, a font or not. A name the rules for a name in Grimoire's folders refuse is not copied, and the
// toast names the rule.
fn a_name_taken_and_a_name_the_rules_refuse() {
    let (root, g) = tree("names");
    let fonts = format!("{g}/fonts");
    std::fs::write(format!("{fonts}/Serif.ttf"), b"the owner's own file of that name").unwrap();
    let mut a = screen();
    for _ in 0..2 {
        let mut b = pick_in(&mut a, "fonts");
        check!(add(&mut b, &mut a, "Serif.ttf").0 == BrowserResult::Copying(CopyTo::Fonts));
        check!(settle() == 1);
        b.close();
    }
    check!(names(&fonts) == ["Serif 2.ttf", "Serif 3.ttf", "Serif.ttf"]);
    check!(std::fs::read(format!("{fonts}/Serif.ttf")).unwrap() == b"the owner's own file of that name");
    check!(std::fs::read(format!("{fonts}/Serif 2.ttf")).unwrap() == good());
    check!(job() == 0);
    let l = listed();
    check!(state_of(&l, "Serif.ttf") == "unreadable" && state_of(&l, "Serif 2.ttf") == "ready" && state_of(&l, "Serif 3.ttf") == "ready");
    // names the rules refuse
    let d = format!("{root}/documents/Type");
    for (name, says) in [(" lead.ttf", "Name can't start or end with a space."), ("a:b.ttf", "Name can't contain :")] {
        std::fs::write(format!("{d}/{name}"), good()).unwrap();
        let mut b = pick_in(&mut a, "documents/Type");
        let (said, toast) = add(&mut b, &mut a, name);
        check!(said == BrowserResult::None && toast == says);
        check!(!bg_busy());
        b.close();
    }
    check!(names(&fonts).len() == 4 && names(&fonts)[0] == ".cache");
    canvas_free(&mut a.screen);
}

// The Kindle's own fonts folder is read only: the storage layer renames and trashes nothing there, a copy out of it
// leaves it as it was, and nothing of Grimoire's is ever written into it (no cache, no mark, no temporary file).
fn the_kindles_fonts_folder_is_read_only() {
    let (root, g) = tree("kindle");
    let kf = format!("{root}/fonts");
    let before = snapshot(&kf);
    let st = FsStorage::open(&root, &g).unwrap();
    for n in ["Serif.ttf", "Serif-Bold.ttf"] {
        let p = sp(&format!("fonts/{n}"));
        let s = st.stat(&p, false).unwrap();
        check!(!st.can_write(&p));
        check!(st.rename(&p, &s, b"Mine.ttf") == Err(FsWriteErr::Refused));
        check!(st.trash(&p, &s) == Err(FsWriteErr::Refused));
    }
    let mut a = screen();
    for n in ["Serif.ttf", "Serif-Bold.ttf"] {
        let mut b = pick_in(&mut a, "fonts");
        check!(add(&mut b, &mut a, n).0 == BrowserResult::Copying(CopyTo::Fonts));
        check!(settle() == 1);
        b.close();
    }
    check!(job() == 0);
    let l = listed();
    check!(state_of(&l, "Serif.ttf") == "ready" && state_of(&l, "Serif-Bold.ttf") == "ready");
    check!(userfonts_try_load("Serif.ttf", "") == Ok("a Bold file"));
    check!(snapshot(&kf) == before);
    // a folder that is not there is no place of the pick
    std::fs::rename(&kf, format!("{root}/fonts-away")).unwrap();
    let mut b = Browser::default();
    check!(b.open(PickSpec::font(), FilesPrefs { card: true, ..Default::default() }));
    check!(probe(&b, "order") == "documents|All storage");
    b.close();
    canvas_free(&mut a.screen);
}

// After a copy into the folder the app asks for the folder to be looked at (userfonts_added), and the check that
// follows is the one every file there gets, with its rule for a file that may still be written: under 5 s old, it
// waits. So the look is held until the copy is that old, and no look in between is queued (it would only find the
// file too young); then one check passes it. The clock here is the caller's: nothing waits five seconds.
fn the_folder_is_looked_at_once_the_copy_is_old_enough() {
    let (_root, g) = tree("age");
    std::env::remove_var("GRIMOIRE_IMPORT_STABLE");
    let path = format!("{g}/fonts/New.ttf");
    std::fs::write(&path, good()).unwrap();
    let mut a = App { dpi: 300.0, ..Default::default() };
    let now = now_ms();
    userfonts_added(300.0);
    // the Font sheet's look, and a tick before the time: no job
    let l = userfonts_scan(300.0);
    check!(userfonts_checking(&l) == 1 && !bg_busy());
    userfonts_tick(&mut a, now.wrapping_add(4000));
    check!(!bg_busy());
    // the time has come (the file is as old as the clock says): one job, which passes the font
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(6);
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(old).unwrap();
    userfonts_tick(&mut a, now.wrapping_add(6000));
    check!(bg_busy());
    settle();
    check!(bg_last_exit() == 0);
    userfonts_tick(&mut a, now.wrapping_add(60_000));
    check!(!bg_busy());
    std::env::set_var("GRIMOIRE_IMPORT_STABLE", "fast");
    check!(state_of(&listed(), "New.ttf") == "ready");
    // where a file is taken as whole at once (the tests' setting), the look comes at the next tick
    std::fs::write(format!("{g}/fonts/Next.ttf"), ttf(&test_font("Bold", 700))).unwrap();
    userfonts_added(300.0);
    check!(!bg_busy());
    userfonts_tick(&mut a, now_ms());
    check!(bg_busy());
    settle();
    check!(state_of(&listed(), "Next.ttf") == "ready");
}

#[test]
fn test_addfont() {
    run!(the_pick_copies_and_only_the_check_makes_a_font);
    run!(a_font_that_fails_its_check_is_listed_and_never_loaded);
    run!(a_copy_cut_short_is_no_font);
    run!(the_source_is_the_file_that_was_listed);
    run!(too_big_and_no_room);
    run!(a_name_taken_and_a_name_the_rules_refuse);
    run!(the_kindles_fonts_folder_is_read_only);
    run!(the_folder_is_looked_at_once_the_copy_is_old_enough);
    std::env::remove_var("GRIMOIRE_STORAGE");
    for n in ["pick", "refused", "cut", "source", "room", "names", "kindle", "age"] {
        let _ = std::fs::remove_dir_all(format!("{}/addfont-{}-{n}", tl_tmp(), std::process::id()));
    }
    done();
}
