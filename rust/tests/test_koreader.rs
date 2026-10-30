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
    run!(wallpaper_and_setting);
    done();
}
