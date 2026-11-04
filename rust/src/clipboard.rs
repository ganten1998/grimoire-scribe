//! clipboard.rs: the lasso's clipboard, `root/clipboard.qpg` (docs/FORMAT.md, The clipboard).
//!
//! A version-1 page file: the size of the page the group came from, Blank paper, the group's strokes
//! at that page's coordinates with its floors encoded as on a page (markers and layer bytes, numbered
//! from 1 in the group), then one critical chunk `CLIP`: where the group came from, its box, and its
//! carried links. One file, written atomically and replaced whole by each Copy and Cut, so it lasts the
//! session: page turns and notebook switches keep it. It does not outlive the session (the owner,
//! 2026-11-04: "yes the lasso clipboard should clear on restart"): every start of Grimoire deletes it
//! (clipboard_clear_at_start, from app_init, before anything can paste), the restart after an update and
//! a start from KOReader too. Only 2.0.0-beta.2 on reads it: older versions never look at files in the data
//! root. It is read quietly (page_deserialize_quiet): its `CLIP` chunk is critical on purpose, which a
//! notebook's page reader would log as a read-only page.

use crate::prelude::*;

/// The chunk's tag (critical: uppercase).
pub const CLIP_TAG: &[u8; 4] = b"CLIP";
/// Its format byte.
pub const CLIP_FORMAT: u8 = 1;
/// A clipboard larger than this, or with more strokes, is refused (it could not come from a page).
pub const CLIP_MAX_BYTES: usize = 16 << 20;
pub const CLIP_MAX_STROKES: usize = 20_000;

pub fn clipboard_path() -> String {
    format!("{}/clipboard.qpg", store_root())
}

/// Whether there is a clipboard to paste (a file; whether it reads is told at the paste).
pub fn clipboard_has() -> bool {
    std::path::Path::new(&clipboard_path()).is_file()
}

/// The start of a session (app_init, every launch): a clipboard an earlier session left is deleted, so
/// nothing can paste it, and grimoire.log says so in one line. Nothing to delete says nothing.
pub fn clipboard_clear_at_start() {
    let path = clipboard_path();
    match std::fs::remove_file(&path) {
        Ok(()) => qlog!("clipboard: the lasso's clipboard from the last session is deleted (it lasts one session)"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => qlog!("clipboard: %s from the last session could not be deleted (%s)", &path, &e.to_string()),
    }
}

/// The chunk of group g: format, source notebook id (u8 length, bytes), source page id (u32), the box
/// (four i32), then the carried links as links.ini lines keyed by their number from 1 (u32 length, text).
fn clip_chunk(g: &Group) -> Vec<u8> {
    let mut d = Vec::new();
    d.push(CLIP_FORMAT);
    let id = g.src_nb.as_bytes();
    let id = &id[..id.len().min(255)];
    d.push(id.len() as u8);
    d.extend_from_slice(id);
    d.extend_from_slice(&g.src_pid.to_le_bytes());
    for v in [g.box_.x, g.box_.y, g.box_.w, g.box_.h] {
        d.extend_from_slice(&v.to_le_bytes());
    }
    let keyed: Vec<NavLink> = g.links.iter().enumerate().map(|(k, l)| NavLink { pid: k as u32 + 1, ..*l }).collect();
    let text = nav_links_lines(&keyed);
    d.extend_from_slice(&(text.len() as u32).to_le_bytes());
    d.extend_from_slice(text.as_bytes());
    let mut c = Vec::with_capacity(d.len() + 8);
    c.extend_from_slice(CLIP_TAG);
    c.extend_from_slice(&(d.len() as u32).to_le_bytes());
    c.extend_from_slice(&d);
    c
}

/// The bytes of the clipboard file for g, or None when it would be too large.
pub fn clipboard_bytes(g: &Group) -> Option<Vec<u8>> {
    if g.strokes.len() > CLIP_MAX_STROKES || g.w <= 0 || g.h <= 0 {
        return None;
    }
    let mut p = page_new(g.w, g.h, PAPER_BLANK);
    p.strokes = g.strokes.clone();
    p.tail = clip_chunk(g);
    let mut out = Vec::new();
    if page_serialize(&p, &mut out) != 0 || out.len() > CLIP_MAX_BYTES {
        return None;
    }
    Some(out)
}

/// Writes g as the clipboard (atomically: tmp, fsync, rename). -1 when it could not.
pub fn clipboard_write(g: &Group) -> i32 {
    let Some(bytes) = clipboard_bytes(g) else { return -1 };
    write_file_atomic(&clipboard_path(), &bytes)
}

fn take<'a>(d: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    if d.len() < n {
        return None;
    }
    let (a, b) = d.split_at(n);
    *d = b;
    Some(a)
}
fn u32_of(d: &mut &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(take(d, 4)?.try_into().ok()?))
}

/// A clipboard file's bytes read back as a group. None for anything that does not read: not a page,
/// too large, more than CLIP_MAX_STROKES strokes, no `CLIP` chunk or one cut off or malformed, or
/// links that do not parse.
pub fn clipboard_parse(buf: &[u8]) -> Option<Group> {
    if buf.len() > CLIP_MAX_BYTES {
        return None;
    }
    let mut out = None;
    // (quietly: the page reader would log the CLIP chunk, critical on purpose, as a read-only page)
    if page_deserialize_quiet(buf, &mut out) != 0 {
        return None;
    }
    let p = out?;
    if p.strokes.len() > CLIP_MAX_STROKES || p.strokes.iter().any(|s| (s.tool.0 as u32) >= TOOL_COUNT as u32) {
        return None;
    }
    // the chunks after the strokes: exactly one, CLIP
    let mut t: &[u8] = &p.tail;
    if take(&mut t, 4)? != CLIP_TAG {
        return None;
    }
    let len = u32_of(&mut t)? as usize;
    if t.len() != len {
        return None;
    }
    let mut d = t;
    if take(&mut d, 1)?[0] != CLIP_FORMAT {
        return None;
    }
    let n = take(&mut d, 1)?[0] as usize;
    let id = std::str::from_utf8(take(&mut d, n)?).ok()?.to_string();
    if !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
        return None;
    }
    let src_pid = u32_of(&mut d)?;
    let mut v = [0i32; 4];
    for x in v.iter_mut() {
        *x = u32_of(&mut d)? as i32;
    }
    let tl = u32_of(&mut d)? as usize;
    let text = std::str::from_utf8(take(&mut d, tl)?).ok()?;
    if !d.is_empty() {
        return None;
    }
    let links = nav_links_from_lines(text)?;
    let ink = p.strokes.iter().filter(|s| stroke_is_ink(s)).count();
    let carried = p.strokes.iter().filter(|s| stroke_is_erase(s) && !is_floor_marker(s)).count();
    Some(Group {
        w: p.w,
        h: p.h,
        strokes: p.strokes,
        links,
        box_: Rect { x: v[0], y: v[1], w: v[2], h: v[3] },
        src_nb: id,
        src_pid,
        ink,
        carried,
    })
}

/// The clipboard file as it is now (its bytes, or None when there is none), so a Cut that copied and then
/// could not take its selection off restores it with clipboard_restore: a refused Cut changes nothing.
pub fn clipboard_snapshot() -> Option<Vec<u8>> {
    let path = clipboard_path();
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() as usize > CLIP_MAX_BYTES {
        return None;
    }
    std::fs::read(&path).ok()
}

/// Puts back a clipboard_snapshot: its bytes (atomically), or no clipboard at all. -1 when it could not.
pub fn clipboard_restore(old: Option<&[u8]>) -> i32 {
    let path = clipboard_path();
    match old {
        Some(b) => write_file_atomic(&path, b),
        None => match std::fs::remove_file(&path) {
            Ok(()) => 0,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
            Err(_) => -1,
        },
    }
}

/// The clipboard read, or None when there is none or it does not read.
pub fn clipboard_read() -> Option<Group> {
    let path = clipboard_path();
    let meta = std::fs::metadata(&path).ok()?;
    if meta.len() as usize > CLIP_MAX_BYTES {
        return None;
    }
    clipboard_parse(&std::fs::read(&path).ok()?)
}
