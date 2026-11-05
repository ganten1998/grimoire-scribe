//! KOReader discovery and optional integration files. No KOReader settings are executed.

use crate::assets::{FONT_TERMINESS_BOLD, FONT_TERMINESS_REGULAR};
use crate::prelude::*;

const FILES: [&str; 4] = ["grimoire.koplugin", "2-no-diagonal-screenshot.lua", "2-mosaic-square-corners.lua", "2-terminess-ui-font.lua"];

/// Grimoire's earlier Terminess patches, byte for byte, as the app carries them in app/koreader/legacy
/// (kindle/koreader/legacy): 1.1.0-beta.2's, rc.6's (1.2.0) and rc.7's (2.0.0-beta.0 and beta.1). A copy in
/// KOReader that is one of them is Grimoire's to replace; any other file there is kept.
pub const KOREADER_TERMINESS_EARLIER: [&str; 3] =
    ["2-terminess-ui-font.lua", "2-terminess-ui-font-rc6.lua", "2-terminess-ui-font-rc7.lua"];

/// Grimoire's own copy of Terminess in KOReader, the files the Terminess menu font patch reads:
/// fonts/Grimoire/<name>, with the binary's bytes.
fn fonts() -> [(&'static str, &'static [u8]); 2] {
    [("TerminessNerdFontMono-Regular.ttf", FONT_TERMINESS_REGULAR), ("TerminessNerdFontMono-Bold.ttf", FONT_TERMINESS_BOLD)]
}

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

/// Which of Grimoire's earlier Terminess patches (KOREADER_TERMINESS_EARLIER, in the store's
/// app/koreader/legacy) the file at path is, byte for byte: its name there, or None (no file, or any
/// other bytes). The size first, so a large file is not read for nothing.
fn terminess_earlier(path: &str) -> Option<&'static str> {
    let len = std::fs::metadata(path).ok()?.len();
    let legacy = cformat!("%s/app/koreader/legacy", &store_root());
    let mut data: Option<Vec<u8>> = None;
    for name in KOREADER_TERMINESS_EARLIER {
        let old = cformat!("%s/%s", &legacy, name);
        if std::fs::metadata(&old).map(|m| m.len()).ok() != Some(len) {
            continue;
        }
        if data.is_none() {
            data = Some(read_file(path)?);
        }
        if read_file(&old).is_some_and(|o| Some(&o) == data.as_ref()) {
            return Some(name);
        }
    }
    None
}

fn copy_owned(src: &str, dst: &str, enabled: bool) -> i32 {
    if accessible(dst, libc::F_OK) && !same(src, dst) {
        // Allow an explicit upgrade/removal of our byte-exact earlier Terminess patches.
        // A modified copy or an unrelated patch stays untouched.
        let owned = terminess_earlier(dst).is_some();
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
        for (name, data) in fonts() {
            csnprintf!(&mut font, 1200, "%s/%s", &dir, name);
            if write_file_atomic(&font, data) != 0 {
                return -1;
            }
        }
    }
    copy_owned(&src, &dst, enabled)
}

/// Keeps Grimoire's own copy of Terminess in KOReader (fonts/Grimoire) the binary's, so an update
/// that changes the embedded fonts reaches it without the patch being toggled: each of its two files
/// that differs from the embedded font (its size, then its bytes) is written again whole (a
/// temporary file, then a rename). A file that is not there is written only while the Terminess
/// patch is applied, since the patch reads it; with neither the patch nor the files, nothing is
/// written. No other KOReader file is read or written. For a quiet moment: the main loop calls it
/// once, after the first frame. Returns how many files it wrote, or -1 when a write failed.
pub fn koreader_fonts_refresh() -> i32 {
    let mut ko = String::new();
    if !koreader_find(&mut ko, 800) {
        return 0;
    }
    let patched = koreader_item_installed(3);
    let mut dir = String::new();
    csnprintf!(&mut dir, 1024, "%s/fonts/Grimoire", &ko);
    let mut wrote = 0;
    for (name, data) in fonts() {
        let mut path = String::new();
        csnprintf!(&mut path, 1200, "%s/%s", &dir, name);
        match std::fs::metadata(&path) {
            Ok(m) if m.len() == data.len() as u64 && read_file(&path).as_deref() == Some(data) => continue,
            Ok(_) => {}
            Err(_) if !patched => continue,
            Err(_) => {}
        }
        if mkdir_p(&dir) != 0 || write_file_atomic(&path, data) != 0 {
            qlog!("koreader: could not bring %s up to date", &path);
            return -1;
        }
        wrote += 1;
    }
    if wrote > 0 {
        qlog!("koreader: wrote %d of Grimoire's Terminess files in %s (they differed from this build's)", wrote, &dir);
    }
    wrote
}

/// Brings an earlier Grimoire's Terminess patch in KOReader up to date, so its fixes reach the menus
/// without the patch being toggled (the owner's Scribe kept rc.6's, whose menus showed a "JS" badge for
/// the battery, 2026-11-04): a patches/2-terminess-ui-font.lua that is byte for byte one of
/// KOREADER_TERMINESS_EARLIER (terminess_earlier: the size, then the bytes) is written again whole with
/// the app's (app/koreader/patches; a temporary file, then a rename), and grimoire.log says so. A copy
/// someone changed, an unrelated file and this build's own copy are left alone, and with the patch not
/// applied nothing is written. No other KOReader file is read or written. For the quiet moment of
/// koreader_fonts_refresh, right after it. Returns 1 when it wrote, 0 when there was nothing to do, or -1
/// when the write failed.
pub fn koreader_patch_refresh() -> i32 {
    let mut src = String::new();
    let mut dst = String::new();
    if !paths(3, &mut src, &mut dst) || !accessible(&dst, libc::F_OK) {
        return 0;
    }
    let Some(was) = terminess_earlier(&dst) else { return 0 };
    let Some(data) = read_file(&src) else { return 0 };
    if read_file(&dst).as_deref() == Some(&data[..]) {
        return 0; // this build's patch is one of the earlier ones too: nothing to do
    }
    if write_file_atomic(&dst, &data) != 0 {
        qlog!("koreader: could not bring %s up to date", &dst);
        return -1;
    }
    qlog!("koreader: %s was Grimoire's earlier patch (%s in app/koreader/legacy); it is this build's now", &dst, was);
    1
}
