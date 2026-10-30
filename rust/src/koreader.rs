//! KOReader discovery and optional integration files. No KOReader settings are executed.

use crate::assets::{FONT_TERMINESS_BOLD, FONT_TERMINESS_REGULAR};
use crate::prelude::*;

const FILES: [&str; 4] = ["grimoire.koplugin", "2-no-diagonal-screenshot.lua", "2-mosaic-square-corners.lua", "2-terminess-ui-font.lua"];

// access(path, mode) == 0
fn accessible(path: &str, mode: libc::c_int) -> bool {
    let Ok(c) = std::ffi::CString::new(path) else { return false };
    // SAFETY: access reads the NUL-terminated path we own.
    unsafe { libc::access(c.as_ptr(), mode) == 0 }
}

fn installed_at(dir: &str) -> bool {
    let mut p = String::new();
    csnprintf!(&mut p, 1024, "%s/reader.lua", dir);
    accessible(&p, libc::R_OK)
}

// getenv(name), the C's way: unset and not UTF-8 are both None here (the paths are Strings).
fn getenv(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn koreader_find(out: &mut String, cap: usize) -> bool {
    if let Some(host) = getenv("KOREADER_DIR") {
        if !host.is_empty() && installed_at(&host) {
            strlcpy_q(out, &host, cap);
            return true;
        }
    }
    let us = match getenv("GRIMOIRE_US") {
        Some(us) if !us.is_empty() => us,
        _ => "/mnt/us".to_string(),
    };
    const PLACES: [&str; 3] = ["koreader", "extensions/koreader", "documents/koreader"];
    for place in PLACES {
        csnprintf!(out, cap, "%s/%s", &us, place);
        if installed_at(out) {
            return true;
        }
    }
    out.clear();
    false
}

/// 0 plugin, 1 screenshot gestures, 2 square corners, 3 Terminess menus.
pub fn koreader_item_name(item: i32) -> &'static str {
    if (0..4).contains(&item) {
        FILES[item as usize]
    } else {
        ""
    }
}

// Where our file for the item is (src, in the store) and where it goes (dst, in KOReader).
fn paths(item: i32, src: &mut String, dst: &mut String) -> bool {
    let mut ko = String::new();
    if !(0..=3).contains(&item) || !koreader_find(&mut ko, 800) {
        return false;
    }
    let file = FILES[item as usize];
    csnprintf!(src, 1024, "%s/app/koreader/%s/%s", &store_root(), if item != 0 { "patches" } else { "" }, file);
    csnprintf!(dst, 1024, "%s/%s/%s", &ko, if item != 0 { "patches" } else { "plugins" }, file);
    true
}

pub fn koreader_item_installed(item: i32) -> bool {
    let mut src = String::new();
    let mut dst = String::new();
    if !paths(item, &mut src, &mut dst) {
        return false;
    }
    if item == 0 {
        let n = dst.len();
        let mut tail = String::new();
        csnprintf!(&mut tail, 1024 - n, "/main.lua");
        dst.push_str(&tail);
    }
    accessible(&dst, libc::R_OK)
}

fn same(a: &str, b: &str) -> bool {
    match (read_file(a), read_file(b)) {
        (Some(ap), Some(bp)) => ap == bp,
        _ => false,
    }
}

fn unlink(path: &str) -> i32 {
    let Ok(c) = std::ffi::CString::new(path) else { return -1 };
    // SAFETY: unlink reads the NUL-terminated path we own.
    unsafe { libc::unlink(c.as_ptr()) }
}

fn copy_owned(src: &str, dst: &str, enabled: bool) -> i32 {
    if accessible(dst, libc::F_OK) && !same(src, dst) {
        // Allow an explicit upgrade/removal of our byte-exact earlier Terminess patches.
        // A modified copy or an unrelated patch stays untouched.
        let legacy_root=format!("{}/app/koreader/legacy",store_root());
        let owned=["2-terminess-ui-font.lua","2-terminess-ui-font-rc6.lua"].iter().any(|name|same(&format!("{legacy_root}/{name}"),dst));
        let name = src.rfind('/');
        match name {
            Some(at) if &src[at + 1..] == "2-terminess-ui-font.lua" && owned => {}
            _ => return -1,
        }
    }
    if !enabled {
        return if unlink(dst) == 0 || !accessible(dst, libc::F_OK) { 0 } else { -1 };
    }
    let Some(data) = read_file(src) else { return -1 };
    write_file_atomic(dst, &data)
}

fn rmdir(path: &str) {
    let Ok(c) = std::ffi::CString::new(path) else { return };
    // SAFETY: rmdir reads the NUL-terminated path we own; a failure is ignored, as the C's is.
    unsafe { libc::rmdir(c.as_ptr()) };
}

/// Installs only our files. A different file at the same path is kept (returns -1).
pub fn koreader_item_set(item: i32, enabled: bool) -> i32 {
    let mut src = String::new();
    let mut dst = String::new();
    let mut ko = String::new();
    if !paths(item, &mut src, &mut dst) || !koreader_find(&mut ko, 800) {
        return -1;
    }
    let mut dir = String::new();
    csnprintf!(&mut dir, 1024, "%s/%s", &ko, if item != 0 { "patches" } else { "plugins/grimoire.koplugin" });
    if enabled && mkdir_p(&dir) != 0 {
        return -1;
    }
    if item == 0 {
        // This directory belongs to Grimoire. Update the plugin, including older Grimoire versions.
        const PARTS: [&str; 2] = ["main.lua", "_meta.lua"];
        for part in PARTS {
            let mut from = String::new();
            let mut to = String::new();
            csnprintf!(&mut from, 1200, "%s/%s", &src, part);
            csnprintf!(&mut to, 1200, "%s/%s", &dst, part);
            if enabled {
                let Some(data) = read_file(&from) else { return -1 };
                if write_file_atomic(&to, &data) != 0 {
                    return -1;
                }
            } else if unlink(&to) != 0 && accessible(&to, libc::F_OK) {
                return -1;
            }
        }
        if !enabled {
            rmdir(&dst);
        }
        let mut mark = String::new();
        csnprintf!(&mut mark, 1024, "%s/.koplugin-installed", &store_root());
        // An explicit removal stays removed at later Library launches.
        if write_file_atomic(&mark, if enabled { b"managed\n" } else { b"removed\n" }) != 0 {
            return -1;
        }
        return 0;
    }
    if enabled && item == 3 {
        csnprintf!(&mut dir, 1024, "%s/fonts/Grimoire", &ko);
        if mkdir_p(&dir) != 0 {
            return -1;
        }
        let mut font = String::new();
        csnprintf!(&mut font, 1200, "%s/TerminessNerdFontMono-Regular.ttf", &dir);
        if write_file_atomic(&font, FONT_TERMINESS_REGULAR) != 0 {
            return -1;
        }
        csnprintf!(&mut font, 1200, "%s/TerminessNerdFontMono-Bold.ttf", &dir);
        if write_file_atomic(&font, FONT_TERMINESS_BOLD) != 0 {
            return -1;
        }
    }
    copy_owned(&src, &dst, enabled)
}
