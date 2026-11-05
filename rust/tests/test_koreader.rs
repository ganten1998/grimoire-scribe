// test_koreader.c: KOReader discovery and the files Grimoire installs there; the sleep wallpaper
// KOReader leaves and the setting that picks it.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn discovery_and_owned_files() {
    let root = cformat!("%s/ko-manager-%ld", tl_tmp().as_str(), std::process::id() as i64);
    let us = cformat!("%s/us", root.as_str());
    std::env::set_var("GRIMOIRE_US", &us);
    std::env::remove_var("KOREADER_DIR");
    check!(store_init(&root) == 0);
    let mut got = String::new();
    check!(!koreader_find(&mut got, 1200));
    let ko = cformat!("%s/extensions/koreader", us.as_str());
    check!(mkdir_p(&ko) == 0);
    let mut p = cformat!("%s/reader.lua", ko.as_str());
    check!(write_file_atomic(&p, b"KOReader") == 0);
    check!(koreader_find(&mut got, 1200) && got == ko);
    let mut src = cformat!("%s/app/koreader/patches", root.as_str());
    check!(mkdir_p(&src) == 0);
    src = cformat!("%s/app/koreader/patches/%s", root.as_str(), koreader_item_name(1));
    check!(write_file_atomic(&src, b"our patch") == 0);
    check!(!koreader_item_installed(1));
    check!(koreader_item_set(1, true) == 0 && koreader_item_installed(1));
    check!(koreader_item_set(1, true) == 0); // idempotent
    check!(koreader_item_set(1, false) == 0 && !koreader_item_installed(1));
    check!(koreader_item_set(1, false) == 0);
    p = cformat!("%s/patches/%s", ko.as_str(), koreader_item_name(1));
    check!(write_file_atomic(&p, b"someone else's patch") == 0);
    check!(koreader_item_set(1, true) != 0);
    check!(koreader_item_set(1, false) != 0);
    let data = read_file(&p);
    check!(data.as_deref() == Some(&b"someone else's patch"[..]));
    check!(koreader_item_set(-1, true) != 0 && koreader_item_set(4, true) != 0);
    src = cformat!("%s/app/koreader/patches/%s", root.as_str(), koreader_item_name(3));
    check!(write_file_atomic(&src, b"new font patch") == 0);
    p = cformat!("%s/app/koreader/legacy", root.as_str());
    check!(mkdir_p(&p) == 0);
    p = cformat!("%s/app/koreader/legacy/%s", root.as_str(), koreader_item_name(3));
    check!(write_file_atomic(&p, b"old font patch") == 0);
    p = cformat!("%s/patches/%s", ko.as_str(), koreader_item_name(3));
    check!(write_file_atomic(&p, b"old font patch") == 0);
    check!(koreader_item_set(3, true) == 0); // known previous version can upgrade explicitly
    let data = read_file(&p);
    check!(data.as_deref() == Some(&b"new font patch"[..]));
    let rc6=cformat!("%s/app/koreader/legacy/2-terminess-ui-font-rc6.lua",root.as_str());
    check!(write_file_atomic(&rc6,b"rc6 font patch")==0);
    check!(write_file_atomic(&p,b"rc6 font patch")==0);
    check!(koreader_item_set(3,true)==0);
    check!(read_file(&p).as_deref()==Some(&b"new font patch"[..]));
    // 2.0.0-beta.1's copy (rc.7's) is an earlier one since 2.0.0-beta.2: Settings replaces it too
    let rc7 = cformat!("%s/app/koreader/legacy/2-terminess-ui-font-rc7.lua", root.as_str());
    check!(write_file_atomic(&rc7, b"rc7 font patch") == 0);
    check!(write_file_atomic(&p, b"rc7 font patch") == 0);
    check!(koreader_item_set(3, true) == 0);
    check!(read_file(&p).as_deref() == Some(&b"new font patch"[..]));
    check!(write_file_atomic(&p,b"rc6 font patch with owner edits")==0);
    check!(koreader_item_set(3,true)!=0 && koreader_item_set(3,false)!=0);
    check!(read_file(&p).as_deref()==Some(&b"rc6 font patch with owner edits"[..]));
    check!(write_file_atomic(&p,b"new font patch")==0);
    check!(koreader_item_set(3, false) == 0);
    src = cformat!("%s/app/koreader/grimoire.koplugin", root.as_str());
    check!(mkdir_p(&src) == 0);
    src = cformat!("%s/app/koreader/grimoire.koplugin/main.lua", root.as_str());
    check!(write_file_atomic(&src, b"new plugin") == 0);
    src = cformat!("%s/app/koreader/grimoire.koplugin/_meta.lua", root.as_str());
    check!(write_file_atomic(&src, b"metadata") == 0);
    check!(koreader_item_set(0, true) == 0 && koreader_item_installed(0));
    check!(koreader_item_set(0, false) == 0 && !koreader_item_installed(0));
    p = cformat!("%s/.koplugin-installed", root.as_str());
    let data = read_file(&p);
    check!(data.as_deref() == Some(&b"removed\n"[..]));
    std::env::remove_var("GRIMOIRE_US");
}

// Grimoire's own Terminess in KOReader (fonts/Grimoire) follows the binary's: a copy that differs is
// written again, an equal one is left alone, a missing one is made only while the Terminess patch is
// applied, and with neither the patch nor the copy nothing is written. No other file is touched.
fn fonts_follow_the_binary() {
    use std::os::unix::fs::MetadataExt;
    let (regular, bold) = (grimoire::assets::FONT_TERMINESS_REGULAR, grimoire::assets::FONT_TERMINESS_BOLD);
    let root = cformat!("%s/ko-fonts-%ld", tl_tmp().as_str(), std::process::id() as i64);
    let _ = std::fs::remove_dir_all(&root);
    let us = cformat!("%s/us", root.as_str());
    std::env::set_var("GRIMOIRE_US", &us);
    std::env::remove_var("KOREADER_DIR");
    check!(store_init(&root) == 0);
    check!(koreader_fonts_refresh() == 0); // no KOReader
    let ko = cformat!("%s/koreader", us.as_str());
    check!(mkdir_p(&cformat!("%s/fonts", ko.as_str())) == 0 && mkdir_p(&cformat!("%s/patches", ko.as_str())) == 0);
    let reader = cformat!("%s/reader.lua", ko.as_str());
    let other = cformat!("%s/fonts/someone-elses.ttf", ko.as_str());
    check!(write_file_atomic(&reader, b"KOReader") == 0 && write_file_atomic(&other, b"not Grimoire's") == 0);
    let dir = cformat!("%s/fonts/Grimoire", ko.as_str());
    let reg = cformat!("%s/TerminessNerdFontMono-Regular.ttf", dir.as_str());
    let bld = cformat!("%s/TerminessNerdFontMono-Bold.ttf", dir.as_str());
    let patch = cformat!("%s/patches/%s", ko.as_str(), koreader_item_name(3));
    let ino = |p: &str| std::fs::metadata(p).map(|m| m.ino()).unwrap_or(0);
    // neither the patch nor the copy: nothing written, not even the folder
    check!(koreader_fonts_refresh() == 0 && std::fs::metadata(&dir).is_err());
    // the patch applied and no copy: both made, the binary's bytes
    check!(write_file_atomic(&patch, b"the Terminess patch") == 0);
    check!(koreader_fonts_refresh() == 2);
    check!(read_file(&reg).as_deref() == Some(regular) && read_file(&bld).as_deref() == Some(bold));
    // equal: left alone (the same files, not written again)
    let (r0, b0) = (ino(&reg), ino(&bld));
    check!(koreader_fonts_refresh() == 0 && ino(&reg) == r0 && ino(&bld) == b0);
    // an earlier, larger copy (the whole Nerd Font, before 2.0.0-beta.2): written again, alone
    check!(write_file_atomic(&reg, &vec![7u8; 2_665_032]) == 0);
    let r1 = ino(&reg);
    check!(koreader_fonts_refresh() == 1 && ino(&reg) != r1 && ino(&bld) == b0);
    check!(read_file(&reg).as_deref() == Some(regular));
    // the same size with other bytes: written again
    let mut changed = bold.to_vec();
    changed[bold.len() / 2] ^= 0xFF;
    check!(write_file_atomic(&bld, &changed) == 0);
    check!(koreader_fonts_refresh() == 1 && read_file(&bld).as_deref() == Some(bold));
    // the patch removed: a stale copy is still brought up to date, a missing one is not made
    check!(std::fs::remove_file(&patch).is_ok() && std::fs::remove_file(&bld).is_ok());
    check!(write_file_atomic(&reg, b"stale") == 0);
    check!(koreader_fonts_refresh() == 1);
    check!(read_file(&reg).as_deref() == Some(regular) && std::fs::metadata(&bld).is_err());
    // neither again: nothing
    check!(std::fs::remove_file(&reg).is_ok());
    check!(koreader_fonts_refresh() == 0 && std::fs::metadata(&reg).is_err() && std::fs::metadata(&bld).is_err());
    // nothing else touched, no temporary file left
    check!(read_file(&other).as_deref() == Some(&b"not Grimoire's"[..]));
    check!(read_file(&reader).as_deref() == Some(&b"KOReader"[..]));
    check!(std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(1) == 0);
    std::env::remove_var("GRIMOIRE_US");
}

// An earlier Grimoire's Terminess patch in KOReader becomes this build's at the start (koreader_patch_refresh,
// with the app's real files): each of kindle/koreader/legacy byte for byte is replaced, atomically; this
// build's is not written again; a copy someone changed, an unrelated file and an absent patch stay as they
// are; without the app's own patch nothing is written. No other file is touched.
fn earlier_terminess_patch_follows_the_binary() {
    use std::os::unix::fs::MetadataExt;
    const KO_SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../kindle/koreader");
    let root = cformat!("%s/ko-patch-%ld", tl_tmp().as_str(), std::process::id() as i64);
    let _ = std::fs::remove_dir_all(&root);
    let us = cformat!("%s/us", root.as_str());
    std::env::set_var("GRIMOIRE_US", &us);
    std::env::remove_var("KOREADER_DIR");
    check!(store_init(&root) == 0);
    check!(koreader_patch_refresh() == 0); // no KOReader
    let ko = cformat!("%s/koreader", us.as_str());
    check!(mkdir_p(&cformat!("%s/patches", ko.as_str())) == 0);
    let reader = cformat!("%s/reader.lua", ko.as_str());
    let other = cformat!("%s/patches/2-someone-elses.lua", ko.as_str());
    check!(write_file_atomic(&reader, b"KOReader") == 0 && write_file_atomic(&other, b"not Grimoire's") == 0);
    let patch = cformat!("%s/patches/%s", ko.as_str(), koreader_item_name(3));
    let ino = |p: &str| std::fs::metadata(p).map(|m| m.ino()).unwrap_or(0);
    // the app as the package lays it out: app/koreader/patches and app/koreader/legacy
    let current = std::fs::read(cformat!("%s/patches/2-terminess-ui-font.lua", KO_SRC)).unwrap();
    check!(mkdir_p(&cformat!("%s/app/koreader/patches", root.as_str())) == 0);
    check!(mkdir_p(&cformat!("%s/app/koreader/legacy", root.as_str())) == 0);
    let mut earlier: Vec<(String, Vec<u8>)> = Vec::new();
    for e in std::fs::read_dir(cformat!("%s/legacy", KO_SRC)).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        let data = std::fs::read(e.path()).unwrap();
        check!(data != current); // an earlier copy is never this build's
        check!(write_file_atomic(&cformat!("%s/app/koreader/legacy/%s", root.as_str(), name.as_str()), &data) == 0);
        earlier.push((name, data));
    }
    earlier.sort();
    // every earlier copy the app carries is one the code knows, and the other way round
    let mut names: Vec<&str> = earlier.iter().map(|(n, _)| n.as_str()).collect();
    let mut known = KOREADER_TERMINESS_EARLIER.to_vec();
    names.sort();
    known.sort();
    check!(names == known);
    // without the app's own patch (a store from before the patches), nothing is written
    check!(write_file_atomic(&patch, &earlier[0].1) == 0);
    check!(koreader_patch_refresh() == 0 && read_file(&patch).as_deref() == Some(&earlier[0].1[..]));
    check!(write_file_atomic(&cformat!("%s/app/koreader/patches/2-terminess-ui-font.lua", root.as_str()), &current) == 0);
    // the patch not applied: nothing written, and none made
    check!(std::fs::remove_file(&patch).is_ok());
    check!(koreader_patch_refresh() == 0 && std::fs::metadata(&patch).is_err());
    // each earlier copy, byte for byte: replaced whole (a new file), then left alone
    for (name, data) in &earlier {
        check!(write_file_atomic(&patch, data) == 0);
        let before = ino(&patch);
        let replaced = koreader_patch_refresh() == 1;
        check!(replaced && read_file(&patch).as_deref() == Some(&current[..]) && ino(&patch) != before);
        if !replaced {
            eprintln!("  the earlier copy {name} was not replaced");
        }
        let after = ino(&patch);
        check!(koreader_patch_refresh() == 0 && ino(&patch) == after);
    }
    // a copy someone changed (one byte more, or one byte different) is kept
    let mut edited = earlier[0].1.clone();
    edited.extend_from_slice(b"-- the owner's line\n");
    check!(write_file_atomic(&patch, &edited) == 0);
    check!(koreader_patch_refresh() == 0 && read_file(&patch).as_deref() == Some(&edited[..]));
    let mut flipped = earlier[1].1.clone();
    flipped[100] ^= 0x20;
    check!(write_file_atomic(&patch, &flipped) == 0);
    check!(koreader_patch_refresh() == 0 && read_file(&patch).as_deref() == Some(&flipped[..]));
    // an unrelated patch at the same name is kept
    check!(write_file_atomic(&patch, b"someone else's Terminess patch") == 0);
    check!(koreader_patch_refresh() == 0 && read_file(&patch).as_deref() == Some(&b"someone else's Terminess patch"[..]));
    // this build's copy: not written again
    check!(write_file_atomic(&patch, &current) == 0);
    let same = ino(&patch);
    check!(koreader_patch_refresh() == 0 && ino(&patch) == same);
    // nothing else touched, no temporary file left
    check!(read_file(&other).as_deref() == Some(&b"not Grimoire's"[..]));
    check!(read_file(&reader).as_deref() == Some(&b"KOReader"[..]));
    check!(std::fs::read_dir(cformat!("%s/patches", ko.as_str())).map(|d| d.count()).unwrap_or(0) == 2);
    check!(std::fs::metadata(cformat!("%s/fonts", ko.as_str())).is_err());
    std::env::remove_var("GRIMOIRE_US");
}

fn wallpaper_and_setting() {
    let root = cformat!("%s/ko-wallpaper-%ld", tl_tmp().as_str(), std::process::id() as i64);
    check!(store_init(&root) == 0);
    let mut s = Settings::default();
    let mut t = Settings::default();
    settings_default(&mut s);
    s.sleep_mode = 3;
    check!(settings_save(&s) == 0 && settings_load(&mut t) == 0 && t.sleep_mode == 3);
    let mut input = Canvas::default();
    let mut out = Canvas::default();
    check!(canvas_init(&mut input, 20, 30) == 0);
    canvas_fill(&mut input, Rect { x: 0, y: 0, w: 20, h: 30 }, 83);
    let path = cformat!("%s/koreader-sleep.png", root.as_str());
    let picture = BgPicture { px: input.px.clone(), w: 20, h: 30, ch: 1 };
    check!(bgimage_write_png(&path, &picture) == 0);
    check!(store_load_koreader_sleep(&mut out) == 0 && out.w == 20 && out.h == 30 && out.px[0] == 83);
    canvas_free(&mut out);
    canvas_free(&mut input);
    check!(write_file_atomic(&path, b"not an image") == 0);
    check!(store_load_koreader_sleep(&mut out) != 0 && out.px.is_empty());
}

#[test]
fn test_koreader() {
    run!(discovery_and_owned_files);
    run!(fonts_follow_the_binary);
    run!(earlier_terminess_patch_follows_the_binary);
    run!(wallpaper_and_setting);
    done();
}
