//! The About sheet, "What's new": every version's notes (docs/whatsnew/VERSION.md, built in) and the
//! owner's signature. The bat in the Library's top bar opens it (library.rs), and so does the first
//! start after every update, once (the owner's rule of 2026-11-05: "no don't show this again, just a
//! "Dismiss" and every update with changes by default has at least a basic what's new"; store.rs,
//! whatsnew_due). Rust only (2.0.0).
//!
//! It opens as a card hanging from the bat: What's new, the version, this version's notes (each
//! heading with its first line; notes without headings, their bullets and paragraphs), the
//! signature, Read all and Dismiss. The card ends at the last of those that fits whole above the
//! signature (card_rows). Read all opens the list of every version's notes on the Library's sheet
//! (the owner, 2026-11-06: "wire read all button to a list of each UNIQUE release note, and allow
//! users to easily browse the changes by version"): a row for each, newest first, with its version
//! (this version's marked), its date and its first line, a page at a time when they need more than
//! one. A tap on a row shows that version's notes whole on one screen (the owner, 2026-11-07: "fit
//! all"): set closer first, then smaller (note_fit), with Back to the list; Dismiss closes What's new
//! from any of them (2.1.0).
//!
//! The notes, a file for each version (2.1.0): build.rs lists docs/whatsnew/*.md, each named for its
//! version (VERSION.md), with the date CHANGELOG.md's "## VERSION (YYYY-MM-DD)" heading gives it, into
//! NOTE_FILES; a new release's notes are a new file there, and no code changes. The archive (notes)
//! puts them newest first by version (update_version_cmp: 2.1.0, 2.0.0, 2.0.0-rc.2, 2.0.0-beta.2,
//! 2.0.0-beta.1, the numbers compared as numbers), one entry for each note: versions whose notes are
//! the same, block for block, share one entry ("2.0.0, 2.0.0-beta.2") at the newest one's place, with
//! its date. 2.0.0-beta.2 has no file: it shipped 2.0.0-beta.1's notes, and its changes are in
//! 2.0.0's. The card's notes are this version's (GRIMOIRE_VERSION), or the newest when this version
//! has none (current).
//!
//! A note is a small Markdown: "# " the title, "## " a heading, "- " a bullet, other lines a
//! paragraph (lines joined until a blank one), and a line "{signature}" where the signature goes.
//! The notes are the owner's text and are not translated; the sheet's own words are.

use crate::prelude::*;
use std::cell::RefCell;
use std::sync::OnceLock;

/// One version's notes as built in: docs/whatsnew/VERSION.md, and the date CHANGELOG.md's heading for
/// the version gives it ("2026-11-05"; "unreleased" for "(unreleased)"; "" without one).
#[derive(Clone, Copy, Debug)]
pub struct NoteFile {
    pub version: &'static str,
    pub date: &'static str,
    pub md: &'static str,
}

// NOTE_FILES: build.rs's list of docs/whatsnew, in file name order (never empty).
include!(concat!(env!("OUT_DIR"), "/whatsnew.rs"));

/// One entry of the archive: a note, and every version that has it, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub versions: Vec<&'static str>,
    /// the newest one's date (NoteFile::date)
    pub date: &'static str,
    pub md: &'static str,
}

impl Entry {
    /// Its versions as the list names them: "2.0.0", or "2.0.0, 2.0.0-beta.2" for notes two shared.
    pub fn label(&self) -> String {
        self.versions.join(", ")
    }
}

/// The notes newest first by version, one entry for each note: a note that is another's block for
/// block (whatever its line breaks and blank lines) adds its version to the first one's entry.
pub fn archive(files: &[NoteFile]) -> Vec<Entry> {
    let mut f: Vec<&NoteFile> = files.iter().collect();
    f.sort_by(|a, b| update_version_cmp(b.version, a.version).cmp(&0).then(a.version.cmp(b.version)));
    let mut out: Vec<(Vec<Block>, Entry)> = Vec::new();
    for n in f {
        let bl = about_blocks(n.md);
        match out.iter_mut().find(|(b, _)| *b == bl) {
            Some((_, e)) => e.versions.push(n.version),
            None => out.push((bl, Entry { versions: vec![n.version], date: n.date, md: n.md })),
        }
    }
    out.into_iter().map(|(_, e)| e).collect()
}

/// The built-in notes' archive (archive of NOTE_FILES).
pub fn notes() -> &'static [Entry] {
    static ARCHIVE: OnceLock<Vec<Entry>> = OnceLock::new();
    // (the notes call the device a Kindle, as every text does: i18n.rs, the device's name)
    ARCHIVE.get_or_init(|| {
        let mut entries = archive(NOTE_FILES);
        for e in &mut entries {
            e.md = crate::i18n::i18n_named(e.md);
        }
        entries
    })
}

/// The entry with version's notes, else the newest (0): the card shows a version without notes of
/// its own the newest there are.
pub fn current(entries: &[Entry], version: &str) -> usize {
    entries.iter().position(|e| e.versions.contains(&version)).unwrap_or(0)
}

/// A note's line in the list: its first heading or first line (its title, when it has one).
pub fn summary(md: &str) -> String {
    about_blocks(md)
        .into_iter()
        .find_map(|b| match b {
            Block::Title(t) | Block::Heading(t) | Block::Bullet(t) | Block::Para(t) => Some(t),
            Block::Signature => None,
        })
        .unwrap_or_default()
}

/// A note's date as the list and the sheet show it: the changelog's, or "unreleased" in the
/// language in use.
pub fn date_label(date: &str) -> &str {
    if date == "unreleased" {
        // TRANSLATORS: What's new's list of versions (Read all), in a version's row and above its
        // notes: in place of the date of a version not released yet, a test build (34 px, at the
        // right: at most 400 px).
        tr("unreleased")
    } else {
        date
    }
}

/// The owner's signature, with the bat drawn beside it: a P5 PGM of the page's ink as Grimoire drew
/// it (the notebook "2.0b1 sig", page 1, cut to its ink), laid over the sheet so that only the ink
/// shows. Empty: a line to sign on shows where it goes.
static SIGNATURE: &[u8] = include_bytes!("../../assets/signature.pgm");

/// The signature's place at most max_w by max_h: the signature's own shape, as large as that holds
/// (never larger than drawn); without one, the whole box (the line to sign on).
fn sig_box(max_w: i32, max_h: i32) -> (i32, i32) {
    match pgm(SIGNATURE) {
        Some((sw, sh, _)) => {
            let k = (max_w as f32 / sw as f32).min(max_h as f32 / sh as f32).min(1.0);
            (((sw as f32 * k) as i32).max(1), ((sh as f32 * k) as i32).max(1))
        }
        None => (max_w, max_h),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Title(String),
    Heading(String),
    Bullet(String),
    Para(String),
    Signature,
}

fn flush(para: &mut String, out: &mut Vec<Block>) {
    if !para.is_empty() {
        out.push(Block::Para(std::mem::take(para)));
    }
}

/// A note's blocks, in order.
pub fn about_blocks(md: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut para = String::new();
    for l in md.lines() {
        let t = l.trim();
        if t.is_empty() {
            flush(&mut para, &mut out);
        } else if let Some(x) = t.strip_prefix("## ") {
            flush(&mut para, &mut out);
            out.push(Block::Heading(x.trim().to_string()));
        } else if let Some(x) = t.strip_prefix("# ") {
            flush(&mut para, &mut out);
            out.push(Block::Title(x.trim().to_string()));
        } else if let Some(x) = t.strip_prefix("- ") {
            flush(&mut para, &mut out);
            out.push(Block::Bullet(x.trim().to_string()));
        } else if t == "{signature}" {
            flush(&mut para, &mut out);
            out.push(Block::Signature);
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t);
        }
    }
    flush(&mut para, &mut out);
    out
}

fn md_title(bl: &[Block]) -> Option<&str> {
    bl.iter().find_map(|b| if let Block::Title(t) = b { Some(t.as_str()) } else { None })
}

/// The card's notes: each heading with the first bullet under it (notes without such pairs, a letter:
/// all their bullets and paragraphs, as many as the card has room for), then the signature if the
/// notes have one.
fn highlights(bl: &[Block]) -> Vec<Block> {
    let mut out = Vec::new();
    let mut head = None;
    for b in bl {
        match b {
            Block::Heading(_) => head = Some(b.clone()),
            Block::Bullet(_) => {
                if let Some(h) = head.take() {
                    out.push(h);
                    out.push(b.clone());
                }
            }
            _ => {}
        }
    }
    if out.is_empty() {
        out = bl.iter().filter(|b| matches!(b, Block::Bullet(_) | Block::Para(_))).cloned().collect();
    }
    if bl.contains(&Block::Signature) {
        out.push(Block::Signature);
    }
    out
}

/// Marks a line never starts with (Japanese's closing marks and small kana), and marks it never
/// ends with (the opening ones): no break before the first, nor after the second.
const NO_BREAK_BEFORE: &str = "、。，．・：；？！ー々ゝゞヽヾ）〕］｝〉》」』】〙〗〟’”｠»ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ";
const NO_BREAK_AFTER: &str = "（〔［｛〈《「『【〘〖〝‘“｟«";

/// The text's pieces a line may break between, each with whether a space comes before it: its
/// words, and in Japanese or Chinese, which have no spaces, each character (font.rs cjk), never a
/// closing mark apart from what comes before it nor an opening one from what follows.
fn pieces(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let (mut cur, mut spaced, mut prev) = (String::new(), true, None::<char>);
        for ch in word.chars() {
            if let Some(p) = prev {
                if (crate::font::cjk(ch as u32) || crate::font::cjk(p as u32)) && !NO_BREAK_BEFORE.contains(ch) && !NO_BREAK_AFTER.contains(p) {
                    out.push((std::mem::take(&mut cur), spaced));
                    spaced = false;
                }
            }
            cur.push(ch);
            prev = Some(ch);
        }
        out.push((cur, spaced));
    }
    out
}

/// Lines of text broken at spaces to fit w, and between the characters of Japanese or Chinese
/// (pieces). A word wider than w on its own (a long link) starts a line and is broken between its
/// characters, as many to a line as fit (at least one), so no line is wider than w and nothing is
/// cut short.
fn wrap(f: FontFace, px: f32, text: &str, w: i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for (piece, spaced) in pieces(text) {
        let trial = if line.is_empty() { piece.clone() } else if spaced { format!("{line} {piece}") } else { format!("{line}{piece}") };
        if font_measure(f, px, &trial) <= w {
            line = trial;
            continue;
        }
        if !line.is_empty() {
            out.push(std::mem::take(&mut line));
        }
        let mut rest = piece.as_str();
        while font_measure(f, px, rest) > w {
            let mut cut = rest.chars().next().map_or(rest.len(), char::len_utf8);
            while let Some(c) = rest[cut..].chars().next() {
                if font_measure(f, px, &rest[..cut + c.len_utf8()]) > w {
                    break;
                }
                cut += c.len_utf8();
            }
            out.push(rest[..cut].to_string());
            rest = &rest[cut..];
        }
        line = rest.to_string();
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn line_h(f: FontFace, px: f32, lead: f32) -> i32 {
    ((font_ascent(f, px) + font_descent(f, px)) as f32 * lead).round() as i32
}

#[derive(Clone, Debug)]
enum Ink {
    Text { s: String, f: FontFace, px: f32 },
    /// a bullet's dot, at this times its size on the card
    Dot(f32),
    Sign,
}

#[derive(Clone, Debug)]
struct Item {
    r: Rect,
    ink: Ink,
}

/// One row of the laid-out notes: its items' rects are relative to the row's top.
struct Row {
    h: i32,
    /// space above it, dropped at a page's top and under a heading
    gap: i32,
    /// a heading's last line: it goes to the next page rather than end one alone
    keep_next: bool,
    /// the block it is a line of (its index in the blocks laid out)
    block: usize,
    /// the last line of a block the card may end at: a bullet's, a paragraph's, the signature's (a
    /// heading goes with the block under it)
    end: bool,
    items: Vec<Item>,
}

struct Look {
    body: f32,
    head_f: FontFace,
    head_px: f32,
    lead: f32,
    /// a bullet's text starts this far in
    indent: i32,
    head_gap: i32,
    item_gap: i32,
    para_gap: i32,
    sign_w: i32,
    sign_h: i32,
    /// a bullet's dot, at this times the card's
    dot: f32,
}

fn rows(bl: &[Block], w: i32, k: &Look) -> Vec<Row> {
    let body_lh = line_h(FontFace::Regular, k.body, k.lead);
    let head_lh = line_h(k.head_f, k.head_px, k.lead);
    let text = |s: String, f: FontFace, px: f32, x: i32, lh: i32, w: i32| Item {
        r: Rect::new(x, 0, w, lh),
        ink: Ink::Text { s, f, px },
    };
    let mut out: Vec<Row> = Vec::new();
    let mut after_head = false;
    for (block, b) in bl.iter().enumerate() {
        let first = out.len();
        match b {
            Block::Title(_) => continue, // (the sheet's title)
            Block::Heading(t) => {
                let lines = wrap(k.head_f, k.head_px, t, w);
                let n = lines.len();
                for (i, l) in lines.into_iter().enumerate() {
                    out.push(Row {
                        h: head_lh,
                        gap: if i == 0 { k.head_gap } else { 0 },
                        keep_next: i + 1 == n,
                        block,
                        end: false,
                        items: vec![text(l, k.head_f, k.head_px, 0, head_lh, w)],
                    });
                }
            }
            Block::Bullet(t) => {
                for (i, l) in wrap(FontFace::Regular, k.body, t, w - k.indent).into_iter().enumerate() {
                    let mut items = vec![text(l, FontFace::Regular, k.body, k.indent, body_lh, w - k.indent)];
                    if i == 0 {
                        items.push(Item { r: Rect::new(0, 0, k.indent, body_lh), ink: Ink::Dot(k.dot) });
                    }
                    out.push(Row { h: body_lh, gap: if i == 0 { k.item_gap } else { 0 }, keep_next: false, block, end: false, items });
                }
            }
            Block::Para(t) => {
                for (i, l) in wrap(FontFace::Regular, k.body, t, w).into_iter().enumerate() {
                    out.push(Row {
                        h: body_lh,
                        gap: if i == 0 { k.para_gap } else { 0 },
                        keep_next: false,
                        block,
                        end: false,
                        items: vec![text(l, FontFace::Regular, k.body, 0, body_lh, w)],
                    });
                }
            }
            Block::Signature => {
                // (signed under the closing, at the left as a letter is; the closing never ends a page
                // without it)
                if let Some(prev) = out.last_mut() {
                    prev.keep_next = true;
                }
                let (sw, sh) = sig_box(k.sign_w.min(w), k.sign_h);
                out.push(Row { h: sh, gap: k.para_gap, keep_next: false, block, end: false, items: vec![Item { r: Rect::new(0, 0, sw, sh), ink: Ink::Sign }] });
            }
        }
        if !matches!(b, Block::Heading(_)) && out.len() > first {
            out.last_mut().unwrap().end = true;
        }
        if after_head {
            if let Some(r) = out.get_mut(first) {
                r.gap = 0; // (what follows a heading sits right under it)
            }
        }
        after_head = matches!(b, Block::Heading(_));
    }
    out
}

/// The rows placed on pages of height h (items' rects relative to the page's top left).
fn paginate(rows: &[Row], h: i32) -> Vec<Vec<Item>> {
    let mut pages: Vec<Vec<Item>> = vec![Vec::new()];
    let mut y = 0;
    for (i, row) in rows.iter().enumerate() {
        let mut gap = if y == 0 { 0 } else { row.gap };
        let next = if row.keep_next { rows.get(i + 1).map_or(0, |n| n.gap + n.h) } else { 0 };
        if y > 0 && y + gap + row.h + next > h {
            pages.push(Vec::new());
            y = 0;
            gap = 0;
        }
        for it in &row.items {
            let mut it = it.clone();
            it.r.y += y + gap;
            pages.last_mut().unwrap().push(it);
        }
        y += gap + row.h;
    }
    pages
}

/// How many of the rows the card shows in room (px), and whether the last of them leaves its block
/// cut: every row of each block up to the last that fits whole (a heading counts with the block under
/// it), so a bullet or paragraph that would be cut is left for Read all, and so is all that follows
/// it. When not even the first block fits whole (a paragraph longer than the card's room), the card
/// shows the lines of it that fit, as a page of Read all would end (at least one; a heading's last
/// line never alone at the bottom): cut at a line, never inside one, and never an empty card, since
/// Read all is a tap away for the rest.
fn card_rows(rs: &[Row], room: i32) -> (usize, bool) {
    let (mut y, mut whole) = (0, 0);
    for (i, r) in rs.iter().enumerate() {
        y += r.h + if i == 0 { 0 } else { r.gap };
        if y > room {
            break;
        }
        if r.end {
            whole = i + 1;
        }
    }
    if whole > 0 || rs.is_empty() {
        return (whole, false);
    }
    let (mut y, mut n) = (0, 0);
    for (i, r) in rs.iter().enumerate() {
        let gap = if i == 0 { 0 } else { r.gap };
        let next = if r.keep_next { rs.get(i + 1).map_or(0, |x| x.gap + x.h) } else { 0 };
        if i > 0 && y + gap + r.h + next > room {
            break;
        }
        y += gap + r.h;
        n = i + 1;
    }
    (n, !rs[n - 1].end)
}

fn rows_h(rows: &[Row]) -> i32 {
    rows.iter().enumerate().map(|(i, r)| r.h + if i == 0 { 0 } else { r.gap }).sum()
}

fn items_h(items: &[Item]) -> i32 {
    items.iter().map(|it| it.r.y + it.r.h).max().unwrap_or(0)
}

fn gothic() -> bool {
    ui_theme() == UiTheme::Gothic
}

fn draw_items(c: &mut Canvas, items: &[Item], at: Rect) {
    for it in items {
        let r = Rect::new(at.x + it.r.x, at.y + it.r.y, it.r.w, it.r.h);
        match &it.ink {
            Ink::Text { s, f, px } => font_draw_left_name(c, *f, *px, r, s, 0),
            Ink::Dot(k) => {
                let (cx, cy) = (r.x as f32 + r.w as f32 * 0.4, r.y as f32 + r.h as f32 / 2.0);
                if gothic() {
                    canvas_lozenge(c, cx, cy, ui_px(10.0 * k) as f32, 0);
                } else {
                    canvas_disc_aa(c, cx, cy, ui_px(8.0 * k) as f32, 0, 1.0);
                }
            }
            Ink::Sign => draw_signature(c, r),
        }
    }
}

/// A P5 PGM's size and pixels (maxval 255, no comments), or None.
fn pgm(d: &[u8]) -> Option<(i32, i32, &[u8])> {
    if !d.starts_with(b"P5") {
        return None;
    }
    let mut at = 2;
    let mut v = [0usize; 3];
    for x in v.iter_mut() {
        while d.get(at)?.is_ascii_whitespace() {
            at += 1;
        }
        let s = at;
        while d.get(at).is_some_and(|c| c.is_ascii_digit()) {
            at += 1;
        }
        *x = std::str::from_utf8(&d[s..at]).ok()?.parse().ok()?;
    }
    let (w, h) = (v[0], v[1]);
    if v[2] != 255 || w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
    // (one white space after the maxval, then the pixels)
    let px = d.get(at + 1..at + 1 + w * h)?;
    Some((w as i32, h as i32, px))
}

/// The signature in r: as large as r holds, at its top left, each pixel the average of the ones it
/// covers, laid over what is there (the darker wins). Without one, a line to sign on.
fn draw_signature(c: &mut Canvas, r: Rect) {
    let Some((sw, sh, px)) = pgm(SIGNATURE) else {
        let base = r.y + r.h - ui_px(20.0);
        canvas_hline(c, r.x, r.x + r.w, base, ui_px(3.0), 0);
        let (x, y, s) = (r.x as f32 + ui_px(14.0) as f32, base as f32 - ui_px(28.0) as f32, ui_px(12.0) as f32);
        canvas_line_aa(c, x - s, y - s, x + s, y + s, ui_px(3.0) as f32, 0);
        canvas_line_aa(c, x - s, y + s, x + s, y - s, ui_px(3.0) as f32, 0);
        font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect::new(r.x, base - ui_px(80.0), r.w, ui_px(70.0)), "signature", 150);
        return;
    };
    let k = (r.w as f32 / sw as f32).min(r.h as f32 / sh as f32).min(1.0);
    let (dw, dh) = (((sw as f32 * k) as i32).max(1), ((sh as f32 * k) as i32).max(1));
    let (dx, dy) = (r.x, r.y);
    let (x0c, y0c) = (c.clip.x.max(0), c.clip.y.max(0));
    let (x1c, y1c) = ((c.clip.x + c.clip.w).min(c.w), (c.clip.y + c.clip.h).min(c.h));
    for y in 0..dh {
        let (y0, y1) = (y * sh / dh, ((y + 1) * sh / dh).max(y * sh / dh + 1));
        for x in 0..dw {
            let (cx, cy) = (dx + x, dy + y);
            if cx < x0c || cy < y0c || cx >= x1c || cy >= y1c {
                continue;
            }
            let (x0, x1) = (x * sw / dw, ((x + 1) * sw / dw).max(x * sw / dw + 1));
            let mut sum = 0u32;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    sum += px[(yy * sw + xx) as usize] as u32;
                }
            }
            let g = (sum / ((y1 - y0) * (x1 - x0)) as u32) as u8;
            let i = (cy * c.stride + cx) as usize;
            if g < c.px[i] {
                c.px[i] = g;
            }
        }
    }
}

/// What's new's three views: the card hanging from the bat (this version's notes), Read all's list
/// of every version's notes, and one version's notes whole on the sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AboutView {
    #[default]
    Card,
    List,
    Note,
}

/// Where the card's or sheet's controls are (empty when not shown), and what it shows (docs/SIM.md,
/// library.about.*): its pages and the page shown; the list's rows on that page, each with its entry
/// in notes(); whether the page shown has the signature; for the card, the notes' blocks it shows, of
/// how many, and whether the last of them is cut; and where the card or the sheet is (a step on it
/// refreshes there alone).
#[derive(Clone, Debug, Default)]
pub struct AboutHits {
    pub close: Rect,
    pub prev: Rect,
    pub next: Rect,
    pub more: Rect,
    pub back: Rect,
    pub rows: Vec<(Rect, usize)>,
    /// the row marked as this version's, on the page shown (from 1; 0 none)
    pub marked: i32,
    pub signed: bool,
    pub area: Rect,
    pub pages: i32,
    /// the page shown (from 0), kept in range
    pub page: i32,
    pub card_blocks: i32,
    pub card_notes: i32,
    pub card_cut: bool,
    /// a version's notes: the size of their body text and their leading (note_fit); else 0
    pub body_px: f32,
    pub lead: f32,
}

/// Draws What's new as view: the card hanging from the bat (bat: its rect in the top bar), Read all's
/// list at page, or the notes of entry note (its index in notes()) at page. Pages count from 0 and are
/// kept in range.
pub fn about_draw(c: &mut Canvas, aw: i32, ah: i32, bat: Rect, view: AboutView, note: usize, page: i32) -> AboutHits {
    let all = notes();
    match view {
        AboutView::Card => draw_card(c, aw, ah, bat, &about_blocks(all[current(all, GRIMOIRE_VERSION)].md)),
        AboutView::List => draw_list(c, aw, ah, all, GRIMOIRE_VERSION, page),
        AboutView::Note => draw_note(c, aw, ah, &all[note.min(all.len() - 1)], page),
    }
}

fn whats_new() -> &'static str {
    // TRANSLATORS: the What's new card's title (520 px at 44 px in bold), and the title of Read all's
    // list of every version's notes and of a version's notes that have no title of their own (the
    // sheet's title type): what this version brings.
    tr("What's new")
}

fn close_label() -> &'static str {
    // TRANSLATORS: What's new's button that closes it, on the card, on Read all's list and on a
    // version's notes (a word under an icon, at 34 px: 200 px). What's new opens by itself once after
    // every update and from the bat in the Library's top bar; this only closes it until then (it is
    // not "never again").
    c_("what's new", "Dismiss")
}

/// The card's measures for aw by ah with the bat at bat, before anything is drawn: the notes it shows
/// (card_rows) and the room they had.
struct Card {
    k: Look,
    pad: i32,
    w: i32,
    iw: i32,
    head_h: i32,
    btn_h: i32,
    top: i32,
    fixed: i32,
    /// the highlights but the signature, laid out; the card shows the first n
    rs: Vec<Row>,
    n: usize,
    cut: bool,
    /// the blocks the card shows, of notes
    blocks: usize,
    notes: usize,
    room: i32,
    sig: bool,
    sw: i32,
    sh: i32,
    sig_h: i32,
}

fn card_layout(aw: i32, ah: i32, bat: Rect, bl: &[Block]) -> Card {
    let pad = ui_px(40.0);
    let w = ui_px(1120.0).min(aw - 2 * ui_px(40.0));
    let iw = w - 2 * pad;
    let k = Look {
        body: ui_small_size(),
        head_f: FontFace::Bold,
        head_px: ui_text_size() * 0.9,
        lead: 1.3,
        indent: ui_px(44.0),
        head_gap: ui_px(26.0),
        item_gap: ui_px(4.0),
        para_gap: ui_px(16.0),
        sign_w: ui_px(560.0),
        sign_h: ui_px(130.0),
        dot: 1.0,
    };
    let head_h = ui_px(84.0);
    let btn_h = ui_px(110.0);
    let top = bat.y + bat.h + ui_px(44.0);
    let fixed = 2 * pad + head_h + ui_px(10.0) + ui_px(30.0) + btn_h;
    // The highlights the screen has room for, each whole (Read all has the rest), the signature always:
    // the card ends above the Library's pager, whose left button and Page 1 of 1 its edge would cover
    // (sideways; 2.1.0).
    let short = highlights(bl);
    let sig = short.last() == Some(&Block::Signature);
    let notes = &short[..short.len() - sig as usize];
    let (sw, sh) = sig_box(k.sign_w, k.sign_h);
    let sig_h = if sig { k.para_gap + sh } else { 0 };
    let room = (pager_top(ah) - top - fixed - sig_h).max(0);
    let rs = rows(notes, iw, &k);
    let (n, cut) = card_rows(&rs, room);
    let blocks = rs[..n].windows(2).filter(|p| p[0].block != p[1].block).count() + (n > 0) as usize;
    Card { k, pad, w, iw, head_h, btn_h, top, fixed, rs, n, cut, blocks, notes: notes.len(), room, sig, sw, sh, sig_h }
}

/// The card: What's new and the version, the highlights that fit whole and the signature, Read all
/// and Dismiss. It hangs from the bat, a notch pointing up at it.
fn draw_card(c: &mut Canvas, aw: i32, ah: i32, bat: Rect, bl: &[Block]) -> AboutHits {
    let l = card_layout(aw, ah, bat, bl);
    let mut hits = AboutHits { pages: 1, card_blocks: l.blocks as i32, card_notes: l.notes as i32, card_cut: l.cut, signed: l.sig, ..AboutHits::default() };
    let (pad, w, iw, head_h, btn_h, k) = (l.pad, l.w, l.iw, l.head_h, l.btn_h, &l.k);
    let page = paginate(&l.rs[..l.n], i32::MAX).swap_remove(0);
    let notes_h = items_h(&page);
    debug_assert!(l.n <= 1 || notes_h <= l.room, "the card's notes overflow their room");
    let card = Rect::new(bat.x.max(ui_px(40.0)).min(aw - w - ui_px(40.0)), l.top, w, l.fixed + notes_h + l.sig_h);
    ui_frame(c, card, ui_px(20.0), 255, true, 0);
    ui_frame(c, card, ui_px(20.0), 0, false, ui_px(5.0));
    let nx = bat.x + bat.w / 2;
    let (nh, t) = (ui_px(30.0), ui_px(7.0));
    for i in 0..nh {
        canvas_hline(c, nx - (nh - i), nx + (nh - i), card.y - i, 1, 0);
    }
    for i in 0..nh - t {
        canvas_hline(c, nx - (nh - t - i), nx + (nh - t - i), card.y + ui_px(5.0) - i, 1, 255);
    }
    hits.area = rect_union(card, Rect::new(nx - nh, card.y - nh, 2 * nh + 1, nh + 1));
    let inner = Rect::new(card.x + pad, card.y + pad, iw, card.h - 2 * pad);
    font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(inner.x, inner.y, iw * 2 / 3, head_h), whats_new(), 0);
    let ver = GRIMOIRE_VERSION;
    let vw = font_measure(FontFace::Regular, k.body, ver);
    font_draw_left_name(c, FontFace::Regular, k.body, Rect::new(inner.x + iw - vw, inner.y, vw, head_h), ver, 90);
    canvas_hline(c, inner.x, inner.x + iw, inner.y + head_h, ui_px(2.0), 0);
    let body = Rect::new(inner.x, inner.y + head_h + ui_px(10.0), iw, notes_h);
    draw_items(c, &page, body);
    if l.sig {
        draw_signature(c, Rect::new(inner.x, body.y + notes_h + k.para_gap, l.sw, l.sh));
    }
    let by = inner.y + inner.h - btn_h;
    let cw = ui_px(260.0);
    hits.close = Rect::new(inner.x + iw - cw, by, cw, btn_h);
    ui_button(c, hits.close, IconKind::Close, close_label(), ButtonStyle::Primary);
    let mw = ui_px(330.0);
    hits.more = Rect::new(hits.close.x - ui_px(24.0) - mw, by, mw, btn_h);
    // TRANSLATORS: the What's new card's button to the list of every version's notes (330 px at 44 px
    // in bold).
    ui_button(c, hits.more, IconKind::None, tr("Read all"), ButtonStyle::Plain);
    hits
}

// ---- the Library's sheet: Read all's list, and a version's notes ----

/// The sheet's width on a screen aw wide.
fn sheet_w(aw: i32) -> i32 {
    (aw - 2 * ui_px(70.0)).min(ui_px(1800.0))
}

/// What the sheet's edges and title take, across and down: ui_sheet's own (ui_sheet_inner), so a
/// body found to fit is the body the sheet has at any dpi, its rounding and all.
fn sheet_chrome() -> (i32, i32) {
    let card = Rect::new(0, 0, 4096, 4096);
    let inner = ui_sheet_inner(card, true);
    (card.w - inner.w, card.h - inner.h)
}

/// The band under the title (the bat and the version), and the foot (the buttons).
fn head_h() -> i32 {
    ui_px(70.0)
}
fn foot_h() -> i32 {
    ui_px(120.0)
}

/// The sheet's height besides its body.
fn sheet_fixed() -> i32 {
    sheet_chrome().1 + head_h() + ui_px(24.0) + ui_px(36.0) + foot_h()
}

/// The tallest body a sheet on a screen ah high has room for.
fn body_max(ah: i32) -> i32 {
    ah - 2 * ui_px(40.0) - sheet_fixed()
}

/// The sheet for a body body_h high (or as high as the screen has room for): under the top bar, the
/// bat in sight, when the screen has the room (sheet_place keeps it off the top bar and the pager).
fn sheet_rect(aw: i32, ah: i32, body_h: i32) -> Rect {
    let w = sheet_w(aw);
    let h = (sheet_fixed() + body_h).min(ah - 2 * ui_px(40.0));
    Rect::new((aw - w) / 2, (ah - ui_px(40.0) - h).min(sheet_top()).max(ui_px(40.0)), w, h)
}

/// The body in the sheet's inside (ui_sheet's answer).
fn sheet_body(inner: Rect) -> Rect {
    let top = head_h() + ui_px(24.0);
    Rect::new(inner.x, inner.y + top, inner.w, inner.h - top - ui_px(36.0) - foot_h())
}

/// The band under the title: the bat, "Grimoire" and the versions, and the date at the right.
fn draw_head(c: &mut Canvas, inner: Rect, versions: &str, date: &str) {
    let (kb, h, px) = (ui_px(2.0), head_h(), ui_small_size());
    draw_bat(c, inner.x, inner.y + (h - BAT_H * kb) / 2, kb);
    let vx = inner.x + BAT_W * kb + ui_px(24.0);
    let dw = if date.is_empty() { 0 } else { font_measure(FontFace::Regular, px, date) };
    if dw > 0 {
        font_draw_left(c, FontFace::Regular, px, Rect::new(inner.x + inner.w - dw, inner.y, dw, h), date, 90);
    }
    let ver = format!("Grimoire {versions}");
    let vw = inner.x + inner.w - vx - if dw > 0 { dw + ui_px(40.0) } else { 0 };
    font_draw_left_name(c, FontFace::Regular, px, Rect::new(vx, inner.y, vw, h), &ver, 90);
}

/// The foot: Dismiss at the right, the page arrows left of it when there are pages (hits' page and
/// pages), and with back Back at the left.
fn draw_foot(c: &mut Canvas, inner: Rect, hits: &mut AboutHits, back: bool) {
    let r = Rect::new(inner.x, inner.y + inner.h - foot_h(), inner.w, foot_h());
    let cw = ui_px(300.0);
    hits.close = Rect::new(r.x + r.w - cw, r.y, cw, r.h);
    ui_button(c, hits.close, IconKind::Close, close_label(), ButtonStyle::Primary);
    if back {
        hits.back = Rect::new(r.x, r.y, cw, r.h);
        // TRANSLATORS: What's new: the button on a version's notes back to the list of every
        // version's notes (Read all), at the left of the sheet's foot (a word under an icon, at 34 px:
        // 300 px).
        ui_button(c, hits.back, IconKind::Back, tr("Back"), ButtonStyle::Plain);
    }
    if hits.pages > 1 {
        let (bw, tw) = (ui_px(130.0), ui_px(150.0));
        let x = hits.close.x - ui_px(40.0) - 2 * bw - tw;
        hits.prev = Rect::new(x, r.y, bw, r.h);
        hits.next = Rect::new(x + bw + tw, r.y, bw, r.h);
        let st = |on: bool| if on { ButtonStyle::Plain } else { ButtonStyle::Disabled };
        ui_button(c, hits.prev, IconKind::Prev, "", st(hits.page > 0));
        ui_button(c, hits.next, IconKind::Next, "", st(hits.page + 1 < hits.pages));
        font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect::new(x + bw, r.y, tw, r.h), &format!("{} / {}", hits.page + 1, hits.pages), 0);
    }
}

/// Read all's list of n entries on aw by ah: the sheet, its rows' height and the gap between them,
/// how many rows a page holds (as many as the tallest sheet has room for) and how many pages they
/// take. The sheet is as tall as a full page of rows (as the rows when they take one page), so a
/// turn keeps it where it is, and placed as a version's notes' is (sheet_place: never across the
/// Library's top bar or its pager, which a list of 6 versions sideways or 9 upright would end over).
struct ListLayout {
    card: Rect,
    rh: i32,
    gap: i32,
    per_page: usize,
    pages: i32,
}

fn list_layout(aw: i32, ah: i32, n: usize) -> ListLayout {
    let (rh, gap) = (ui_px(170.0), ui_px(20.0));
    let per_page = ((body_max(ah) + gap) / (rh + gap)).max(1) as usize;
    let shown = n.clamp(1, per_page) as i32;
    let card = sheet_place(aw, ah, shown * (rh + gap) - gap);
    ListLayout { card, rh, gap, per_page, pages: n.div_ceil(per_page).max(1) as i32 }
}

/// A row of the list: the versions in bold (this version's marked beside them, white on black), the
/// date at the right, and the notes' first heading or first line under them, cut short to the row.
fn draw_row(c: &mut Canvas, r: Rect, e: &Entry, this: bool) {
    ui_frame(c, r, ui_px(16.0), 255, true, 0);
    ui_frame(c, r, ui_px(16.0), 0, false, ui_px(3.0));
    let (big, small) = (ui_text_size(), ui_small_size());
    let (h1, h2) = (line_h(FontFace::Bold, big, 1.0), line_h(FontFace::Regular, small, 1.0));
    let gap = (r.h - h1 - h2) / 3;
    let (y1, y2) = (r.y + gap, r.y + 2 * gap + h1);
    let pad = ui_px(32.0);
    let (x, right) = (r.x + pad, r.x + r.w - pad);
    let date = date_label(e.date);
    let dw = if date.is_empty() { 0 } else { font_measure(FontFace::Regular, small, date) };
    if dw > 0 {
        font_draw_left(c, FontFace::Regular, small, Rect::new(right - dw, y1, dw, h1), date, 90);
    }
    // TRANSLATORS: What's new's list of versions (Read all): the mark beside the version running,
    // white on a black box in its row (34 px in bold: at most 360 px).
    let mark = if this { tr("This version") } else { "" };
    let (mpad, mgap) = (ui_px(18.0), ui_px(24.0));
    let mw = if this { font_measure(FontFace::Bold, small, mark) + 2 * mpad } else { 0 };
    let room = right - x - if dw > 0 { dw + ui_px(40.0) } else { 0 } - if this { mw + mgap } else { 0 };
    let label = e.label();
    let lw = font_measure(FontFace::Bold, big, &label).min(room.max(1));
    font_draw_left_name(c, FontFace::Bold, big, Rect::new(x, y1, lw, h1), &label, 0);
    if this {
        let mh = h2 + ui_px(8.0);
        let m = Rect::new(x + lw + mgap, y1 + (h1 - mh) / 2, mw, mh);
        ui_frame(c, m, ui_px(10.0), 0, true, 0);
        font_draw_centered(c, FontFace::Bold, small, m, mark, 255);
    }
    font_draw_left_name(c, FontFace::Regular, small, Rect::new(x, y2, right - x, h2), &summary(e.md), 0);
}

/// Read all: the list of every version's notes on the Library's sheet, What's new its title, the bat
/// and this version under it; a row for each entry, newest first, a page of them at a time, this
/// version's marked; Dismiss (and the page arrows left of it). A tap on a row shows its notes.
fn draw_list(c: &mut Canvas, aw: i32, ah: i32, all: &[Entry], version: &str, page: i32) -> AboutHits {
    let l = list_layout(aw, ah, all.len());
    let mut hits = AboutHits { area: l.card, pages: l.pages, page: page.clamp(0, l.pages - 1), ..AboutHits::default() };
    let inner = ui_sheet(c, l.card, whats_new());
    draw_head(c, inner, version, "");
    let body = sheet_body(inner);
    let first = hits.page as usize * l.per_page;
    for (k, (i, e)) in all.iter().enumerate().skip(first).take(l.per_page).enumerate() {
        let r = Rect::new(body.x, body.y + k as i32 * (l.rh + l.gap), body.w, l.rh);
        let this = e.versions.contains(&version);
        draw_row(c, r, e, this);
        if this {
            hits.marked = k as i32 + 1;
        }
        hits.rows.push((r, i));
    }
    draw_foot(c, inner, &mut hits, false);
    hits
}

/// The notes' leading at its most open (the look Read all has always set them in) and at its
/// tightest, and their gaps at their tightest as a share of the open look's (the owner, 2026-11-07:
/// "The note reading itself isn't polished enough"): notes too long for one screen tighten these at
/// their size before their type gets smaller (note_fit).
const LEAD: f32 = 1.3;
const LEAD_MIN: f32 = 1.15;
const GAPS_MIN: f32 = 0.6;
/// The steps from the open look (0) to the tightest: the leading a hundredth a step.
const TIGHT_STEPS: i32 = 15;

/// A bullet's dot beside body text at px, as a share of the card's: in the proportion the card's dot
/// has to the card's text (ui_small_size), by the type's height as it is drawn. Notes at the card's
/// size have the card's dots, larger ones larger dots, and Gothic's type, which changes size only at
/// whole design pixels, the dots of the size it shows.
fn dot_scale(px: f32) -> f32 {
    let h = |px: f32| (font_ascent(FontFace::Regular, px) + font_descent(FontFace::Regular, px)) as f32;
    h(px) / h(ui_small_size()).max(1.0)
}

/// The notes' look with their body text at px, tight steps from the open look to the tightest
/// (TIGHT_STEPS): at ui_text_size and 0, the look Read all has always set them in, but for the
/// bullets' dots (dot_scale). Smaller type makes every measure smaller with it (the headings, the
/// leading's lines, the bullets' indent, the gaps, the signature); each step tighter takes the
/// leading a hundredth nearer LEAD_MIN, and the gaps nearer GAPS_MIN of theirs, at the same size.
fn note_look(px: f32, tight: i32) -> Look {
    let k = px / ui_text_size();
    let t = tight.clamp(0, TIGHT_STEPS) as f32 / TIGHT_STEPS as f32;
    let g = k * (1.0 - (1.0 - GAPS_MIN) * t);
    let at = |v: f32| ui_px(v * k);
    let gap = |v: f32| ui_px(v * g);
    Look {
        body: px,
        head_f: FontFace::Bold,
        head_px: px,
        lead: LEAD - (LEAD - LEAD_MIN) * t,
        indent: at(56.0),
        head_gap: gap(44.0),
        item_gap: gap(8.0),
        para_gap: gap(24.0),
        sign_w: at(720.0),
        sign_h: at(190.0),
        dot: dot_scale(px),
    }
}

/// The sizes note_fit tries below ui_text_size, down to the floor (44 px to 34 px at 300 dpi, half a
/// pixel a step).
const FIT_STEPS: i32 = 20;

/// note_fit's size i: ui_text_size at 0, the floor (ui_small_size) at FIT_STEPS.
fn fit_size(i: i32) -> f32 {
    let (top, floor) = (ui_text_size(), ui_small_size());
    top + (floor - top) * i as f32 / FIT_STEPS as f32
}

/// The width a version's notes are set in on a screen aw wide (the sheet's inside).
fn note_w(aw: i32) -> i32 {
    sheet_w(aw) - sheet_chrome().0
}

/// How a version's notes are set (note_look): their body size, and how many steps tighter than the
/// open look.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fit {
    px: f32,
    tight: i32,
}

/// How notes bl are set on aw by ah to fit the tallest sheet's body whole, on one page (the owner,
/// 2026-11-07: "The read all doesn't need text that big -- either smaller text for all past
/// messages, or fit all (which i lean toward)"; and "The note reading itself isn't polished
/// enough"). At ui_text_size, the size they have always had, the leading and the gaps tighten
/// first; only notes too long even at the tightest step down in size, in FIT_STEPS steps, to the
/// largest size at which the tightest look fits, and the room a step down leaves goes back to the
/// leading and the gaps, up to the open look, so the notes fill the sheet evenly. (Gothic's type has
/// two sizes in that range, ui_text_size and the next down from 43.5 px: a note that needs the
/// smaller is set in it with the room it leaves.) The floor is ui_small_size, the size the card sets
/// the same notes in and the list the rows' lines: when the notes do not fit even there, they are set
/// at the floor in the open look and take pages. Sizes are tried from the top down, a size at a time:
/// a smaller size wraps the lines anew, so a size that fits says nothing of the next. At one size each
/// step tighter is no taller (the lines are the same), so the most open look that fits is halved for.
fn note_fit(bl: &[Block], aw: i32, ah: i32) -> Fit {
    #[cfg(test)]
    FIT_RUNS.set(FIT_RUNS.get() + 1);
    let (w, room) = (note_w(aw), body_max(ah));
    let fits = |px: f32, tight: i32| rows_h(&rows(bl, w, &note_look(px, tight))) <= room;
    for px in (0..=FIT_STEPS).map(fit_size) {
        if !fits(px, TIGHT_STEPS) {
            continue;
        }
        let (mut open, mut tight) = (0, TIGHT_STEPS);
        while open < tight {
            let mid = (open + tight) / 2;
            if fits(px, mid) {
                tight = mid;
            } else {
                open = mid + 1;
            }
        }
        return Fit { px, tight };
    }
    Fit { px: ui_small_size(), tight: 0 }
}

thread_local! {
    /// note_set's answers: the notes (their text's place and length), the screen's width and height,
    /// the look, Classic's font (2.1, Settings > Font; 0 in Gothic, which has one) and the body type's
    /// size, and how note_fit set them.
    static NOTE_FIT: RefCell<Vec<((usize, usize, i32, i32, bool, usize, u32), Fit)>> = const { RefCell::new(Vec::new()) };
}
#[cfg(test)]
thread_local! {
    /// note_fit's runs, for the test that a note's size is found once for each screen
    static FIT_RUNS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// note_fit's answer for the notes md (bl, their blocks) on aw by ah, found once for each note,
/// orientation, look and font, not at every draw. Each of Classic's fonts sets the notes at its own
/// widths, so a size found in one is not the size for another.
fn note_set(md: &'static str, bl: &[Block], aw: i32, ah: i32) -> Fit {
    let family = if gothic() { 0 } else { font_look() };
    let key = (md.as_ptr() as usize, md.len(), aw, ah, gothic(), family, ui_text_size().to_bits());
    if let Some(f) = NOTE_FIT.with_borrow(|v| v.iter().find(|e| e.0 == key).map(|e| e.1)) {
        return f;
    }
    let f = note_fit(bl, aw, ah);
    NOTE_FIT.with_borrow_mut(|v| v.push((key, f)));
    f
}

/// The top of a sheet under the Library's top bar, the bat in sight (sheet_rect's).
fn sheet_top() -> i32 {
    ui_px(214.0)
}

/// The top of the Library's pager at the foot (its rule; library.rs bottom_h).
fn pager_top(ah: i32) -> i32 {
    ah - crate::library::bottom_h() - ui_px(10.0)
}

/// The sheet for Read all's list or a version's notes, body_h high, as sheet_rect places it, but never
/// across the Library's top bar's buttons or its pager, whose icons and label its edge would cut
/// through: a sheet that would start across the top bar's buttons starts where the tallest sheet
/// does, at the margin above them (2.0.0's notes sideways in Classic once they fit, whose edge showed
/// the icons' tops cut off), while one that starts below them keeps its place, the bat in sight; and
/// one that would end over the pager (its rule and its Page 1 of 1: 2.0.0's notes sideways in Gothic)
/// reaches down to the margin above the screen's edge, as the tallest sheet does, the room under what
/// it holds left blank.
fn sheet_place(aw: i32, ah: i32, body_h: i32) -> Rect {
    let mut r = sheet_rect(aw, ah, body_h);
    if r.y < crate::library::top_buttons_foot() {
        r.y = ui_px(40.0);
    }
    if r.y + r.h > pager_top(ah) {
        r.h = ah - ui_px(40.0) - r.y;
    }
    r
}

/// A version's notes whole on the Library's sheet: their title as its title (else What's new), the
/// bat, the version and its date under it, the notes on one page, set as note_fit found (at the floor
/// and a page at a time when even that does not fit them), Back to the list at the foot's left and
/// Dismiss at its right (the page arrows left of Dismiss when there are pages).
fn draw_note(c: &mut Canvas, aw: i32, ah: i32, e: &Entry, page: i32) -> AboutHits {
    let bl = about_blocks(e.md);
    let fit = note_set(e.md, &bl, aw, ah);
    let k = note_look(fit.px, fit.tight);
    let rs = rows(&bl, note_w(aw), &k);
    let card = sheet_place(aw, ah, rows_h(&rs));
    let inner = ui_sheet(c, card, md_title(&bl).unwrap_or(whats_new()));
    draw_head(c, inner, &e.label(), date_label(e.date));
    let body = sheet_body(inner);
    let pages = paginate(&rs, body.h);
    let mut hits = AboutHits { area: card, pages: pages.len() as i32, body_px: fit.px, lead: k.lead, ..AboutHits::default() };
    hits.page = page.clamp(0, hits.pages - 1);
    let shown = &pages[hits.page as usize];
    hits.signed = shown.iter().any(|it| matches!(it.ink, Ink::Sign));
    draw_items(c, shown, body);
    draw_foot(c, inner, &mut hits, true);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_of_the_notes() {
        let b = about_blocks("# T\n\n## H\n- one\n- two\n\nA para\nthat goes on.\n\n{signature}\n");
        assert_eq!(
            b,
            vec![
                Block::Title("T".into()),
                Block::Heading("H".into()),
                Block::Bullet("one".into()),
                Block::Bullet("two".into()),
                Block::Para("A para that goes on.".into()),
                Block::Signature,
            ]
        );
        // Every built-in note has a place for the signature, at its end.
        for e in notes() {
            assert_eq!(about_blocks(e.md).last(), Some(&Block::Signature), "{}", e.label());
        }
    }

    #[test]
    fn the_card_keeps_each_headings_first_line_and_the_signature() {
        let b = about_blocks("# T\n## A\n- a1\n- a2\n## B\nprose\n## C\n- c1\n{signature}\n");
        let h = highlights(&b);
        let want = ["A", "a1", "C", "c1"];
        assert_eq!(h.len(), 5);
        for (got, w) in h.iter().zip(want) {
            match got {
                Block::Heading(t) | Block::Bullet(t) => assert_eq!(t, w),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(h[4], Block::Signature);
        // A letter (no headings): all its bullets and paragraphs, then the signature.
        let h = highlights(&about_blocks("one\n\n- two\n- three\n- four\n- five\n\nCheers,\n\n{signature}\n"));
        assert_eq!(h.len(), 7);
        assert_eq!(h[0], Block::Para("one".into()));
        assert_eq!(h[6], Block::Signature);
    }

    fn file(version: &'static str, date: &'static str, md: &'static str) -> NoteFile {
        NoteFile { version, date, md }
    }

    #[test]
    fn the_archive_is_newest_first_one_entry_a_note() {
        // In no order, as a folder lists them; beta.2 shipped beta.1's notes (another line break, a
        // blank line more), rc.10 2.1.0's.
        let files = [
            file("2.0.0-beta.1", "2026-11-01", "Dear reader,\n\n- one\n- two\n\n{signature}\n"),
            file("2.0.0-rc.2", "", "# Since rc.1\n\n- a fix\n"),
            file("2.1.0", "unreleased", "# What's new (since 2.0.0)\n\n- wake\n\n{signature}\n"),
            file("2.0.0-beta.10", "", "# Ten\n\n- ten\n"),
            file("2.0.0", "2026-11-05", "# What's new (since 2.0.0-beta.1)\n\n- lots\n\n{signature}\n"),
            file("2.0.0-beta.2", "2026-11-05", "Dear\nreader,\n\n\n- one\n- two\n{signature}"),
            file("2.0.0-rc.10", "", "# What's new (since 2.0.0)\n- wake\n\n{signature}"),
        ];
        let a = archive(&files);
        let got: Vec<String> = a.iter().map(|e| e.label()).collect();
        // (by version, numbers as numbers: rc.10 after 2.1.0's place, beta.10 before beta.2)
        assert_eq!(got, ["2.1.0, 2.0.0-rc.10", "2.0.0", "2.0.0-rc.2", "2.0.0-beta.10", "2.0.0-beta.2, 2.0.0-beta.1"]);
        // An entry's date is its newest version's; its notes are that version's text.
        assert_eq!(a[0].date, "unreleased");
        assert_eq!(a[4].date, "2026-11-05");
        assert_eq!(a[4].md, files[5].md);
        // Unique: no two entries have the same notes.
        for (i, x) in a.iter().enumerate() {
            for y in &a[i + 1..] {
                assert_ne!(about_blocks(x.md), about_blocks(y.md));
            }
        }
        assert!(archive(&[]).is_empty());
    }

    #[test]
    fn the_current_note_is_this_versions_else_the_newest() {
        let files = [
            file("2.0.0-beta.1", "2026-11-01", "Letter.\n\n{signature}\n"),
            file("2.0.0", "2026-11-05", "# Since beta.1\n\n- lots\n"),
            file("2.0.0-beta.2", "2026-11-05", "Letter.\n\n{signature}\n"),
        ];
        let a = archive(&files);
        assert_eq!(current(&a, "2.0.0"), 0);
        assert_eq!(current(&a, "2.0.0-beta.2"), 1);
        assert_eq!(current(&a, "2.0.0-beta.1"), 1);
        // A version with no notes of its own (2.1.0 before its file is written) shows the newest.
        assert_eq!(current(&a, "2.1.0"), 0);
        assert_eq!(a[current(&a, "2.1.0")].versions[0], "2.0.0");
        assert_eq!(current(&a, "1.2.0"), 0);
        // The built-in notes: this version has its own (docs/RELEASING.md, The bump commit), and the
        // card shows them.
        assert!(notes()[current(notes(), GRIMOIRE_VERSION)].versions.contains(&GRIMOIRE_VERSION));
    }

    #[test]
    fn the_built_in_notes_are_the_folders() {
        // build.rs's list is docs/whatsnew/*.md, every file, with its text (an editor's files passed
        // over as build.rs passes them: .#2.1.0.md, #2.1.0.md#, 2.1.1.md~).
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/whatsnew");
        let mut want: Vec<(String, String)> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| {
                let p = e.unwrap().path();
                let name = p.file_name()?.to_str()?;
                if name.starts_with('.') || name.starts_with('#') || name.ends_with('~') {
                    return None;
                }
                let v = name.strip_suffix(".md")?.to_string();
                Some((v, std::fs::read_to_string(&p).unwrap()))
            })
            .collect();
        want.sort();
        let mut got: Vec<(String, String)> = NOTE_FILES.iter().map(|f| (f.version.to_string(), f.md.to_string())).collect();
        got.sort();
        assert_eq!(got, want);
        // The released versions' dates are the changelog's; 2.0.0-beta.2 has no notes of its own (it
        // shipped 2.0.0-beta.1's, and 2.0.0's notes have its changes).
        let date = |v: &str| NOTE_FILES.iter().find(|f| f.version == v).map(|f| f.date);
        assert_eq!(date("2.0.0-beta.1"), Some("2026-11-01"));
        assert_eq!(date("2.0.0"), Some("2026-11-05"));
        assert_eq!(date("2.0.0-beta.2"), None);
        for f in NOTE_FILES {
            let b = f.date.as_bytes();
            assert!(f.date.is_empty() || f.date == "unreleased" || (b.len() == 10 && b[4] == b'-' && b[7] == b'-'), "{}", f.date);
        }
        // Newest first, each note once.
        let all = notes();
        for p in all.windows(2) {
            assert!(update_version_cmp(p[0].versions[0], p[1].versions[0]) > 0);
        }
        assert_eq!(all.iter().map(|e| e.versions.len()).sum::<usize>(), NOTE_FILES.len());
    }

    #[test]
    fn a_rows_line_is_the_first_heading_or_line() {
        assert_eq!(summary("# What's new (since 2.0.0)\n\n- wake\n{signature}\n"), "What's new (since 2.0.0)");
        assert_eq!(summary("## Pages\n- drag\n"), "Pages");
        assert_eq!(summary("Grimoire was made\nwith love.\n\n- one\n"), "Grimoire was made with love.");
        assert_eq!(summary("{signature}\n"), "");
        // The letter of 2.0.0-beta.1, which has no title: its first paragraph.
        let beta1 = NOTE_FILES.iter().find(|f| f.version == "2.0.0-beta.1").unwrap();
        assert!(summary(beta1.md).starts_with("Grimoire was made with a lot of love"));
    }

    fn up(theme: UiTheme) {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        ui_set_theme(theme);
        i18n_apply("en");
    }

    /// The bat as the Library's top bar places it (library.rs draw_base: the margin 50 px, the bar
    /// 190 px high, the bat 3 px a unit).
    fn bat() -> Rect {
        let k = ui_px(3.0);
        Rect::new(ui_px(50.0), (ui_px(190.0) - BAT_H * k) / 2, BAT_W * k, BAT_H * k)
    }

    fn screen(aw: i32, ah: i32) -> Canvas {
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, aw, ah), 0);
        c
    }

    fn inside(o: Rect, r: Rect) -> bool {
        rect_contains_rect(o, r)
    }

    /// n entries, 2.n-1.0 down to 2.0.0, each its own note.
    fn entries(n: usize) -> Vec<Entry> {
        (0..n)
            .rev()
            .map(|i| Entry {
                versions: vec![Box::leak(format!("2.{i}.0").into_boxed_str())],
                date: "2026-11-05",
                md: Box::leak(format!("# Notes {i}\n\n- a change\n").into_boxed_str()),
            })
            .collect()
    }

    #[test]
    fn the_list_holds_whole_rows_a_page_at_a_time() {
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                for n in [1, 3, 40] {
                    let all = entries(n);
                    let l = list_layout(aw, ah, n);
                    eprintln!("{theme:?} {aw}x{ah} n {n}: {} a page, {} pages", l.per_page, l.pages);
                    // As many rows a page as the tallest sheet holds, and no more.
                    let fits = |k: i32| k * (l.rh + l.gap) - l.gap <= body_max(ah);
                    assert!(fits(l.per_page as i32) && !fits(l.per_page as i32 + 1));
                    assert_eq!(l.pages as usize, n.div_ceil(l.per_page));
                    let mut seen = Vec::new();
                    for page in 0..l.pages {
                        let mut c = screen(aw, ah);
                        let h = draw_list(&mut c, aw, ah, &all, "2.1.0", page);
                        assert_eq!((h.page, h.pages), (page, l.pages));
                        assert!(!h.rows.is_empty() && h.rows.len() <= l.per_page);
                        for (i, (r, _)) in h.rows.iter().enumerate() {
                            // each row on the sheet, above its buttons, under the one before it
                            assert!(inside(h.area, *r) && r.y + r.h <= h.close.y, "{r:?} {:?}", h.close);
                            if i > 0 {
                                assert!(r.y >= h.rows[i - 1].0.y + h.rows[i - 1].0.h);
                            }
                        }
                        assert!(inside(Rect::new(0, 0, aw, ah), h.area));
                        // the page arrows when there are pages, Dismiss always, no Back on the list
                        assert_eq!(rect_is_empty(h.next), l.pages == 1);
                        assert!(!rect_is_empty(h.close) && rect_is_empty(h.back));
                        // 2.1.0 (the entry before the oldest) marked in its row, on its page alone
                        let at = n.checked_sub(2).filter(|i| i / l.per_page == page as usize);
                        assert_eq!(h.marked, at.map_or(0, |i| (i % l.per_page) as i32 + 1));
                        seen.extend(h.rows.iter().map(|r| r.1));
                    }
                    // every entry once, newest first
                    assert_eq!(seen, (0..n).collect::<Vec<_>>());
                }
                // A few rows: one page, the sheet no taller than they need.
                let l = list_layout(aw, ah, 3);
                assert_eq!(l.pages, 1);
                assert_eq!(l.card.h, sheet_fixed() + 3 * l.rh + 2 * l.gap);
            }
        }
    }

    #[test]
    fn the_built_in_list_marks_this_version() {
        up(UiTheme::Gothic);
        let mut c = screen(1860, 2480);
        let h = draw_list(&mut c, 1860, 2480, notes(), GRIMOIRE_VERSION, 0);
        let all = notes();
        assert_eq!(h.rows.len(), all.len().min(list_layout(1860, 2480, all.len()).per_page));
        let i = current(all, GRIMOIRE_VERSION);
        assert_eq!(h.marked as usize, i + 1);
        // a version that has no notes: no row is its own
        let h = draw_list(&mut c, 1860, 2480, all, "9.9.9", 0);
        assert_eq!(h.marked, 0);
    }

    #[test]
    fn a_versions_notes_end_with_the_signature_and_go_back() {
        let beta1 = notes().iter().find(|e| e.versions.contains(&"2.0.0-beta.1")).unwrap();
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                let mut c = screen(aw, ah);
                let first = draw_note(&mut c, aw, ah, beta1, 0);
                let last = draw_note(&mut c, aw, ah, beta1, first.pages - 1);
                eprintln!("{theme:?} {aw}x{ah}: the letter takes {} pages", first.pages);
                // the signature on the last page, and on no other
                assert!(last.signed);
                assert!(first.pages == 1 || !first.signed);
                // Back at the foot's left, Dismiss at its right, the arrows between when there are pages
                for h in [&first, &last] {
                    assert!(!rect_is_empty(h.back) && h.back.x + h.back.w <= h.close.x);
                    assert!(h.pages == 1 || (h.back.x + h.back.w <= h.prev.x && h.next.x + h.next.w <= h.close.x));
                    assert!(inside(h.area, h.back) && inside(h.area, h.close));
                    assert!(h.rows.is_empty() && rect_is_empty(h.more));
                }
                // a page past the end shows the last
                assert_eq!(draw_note(&mut c, aw, ah, beta1, 99).page, first.pages - 1);
            }
        }
    }

    /// A note of md, as the archive would have it (version 9.9.9, no date).
    fn note(md: String) -> Entry {
        Entry { versions: vec!["9.9.9"], date: "", md: Box::leak(md.into_boxed_str()) }
    }

    /// Whether notes bl fit the tallest sheet on aw by ah whole at body size px, tight steps tighter
    /// than the open look.
    fn fits(bl: &[Block], aw: i32, ah: i32, px: f32, tight: i32) -> bool {
        rows_h(&rows(bl, note_w(aw), &note_look(px, tight))) <= body_max(ah)
    }

    /// The height of the body type at px as it is drawn (Gothic's in whole design pixels).
    fn type_h(px: f32) -> i32 {
        font_ascent(FontFace::Regular, px) + font_descent(FontFace::Regular, px)
    }

    /// That fit is note_fit's way of setting notes bl on aw by ah: the largest size at which their
    /// tightest look fits (at each larger size even that does not), in the most open look that fits
    /// there (a step more open does not); at the floor in the open look when nothing fits.
    fn check_fit(bl: &[Block], aw: i32, ah: i32, fit: Fit, what: &str) {
        let i = (0..=FIT_STEPS).position(|i| fit_size(i) == fit.px).expect("one of note_fit's sizes");
        for j in 0..i as i32 {
            assert!(!fits(bl, aw, ah, fit_size(j), TIGHT_STEPS), "{what}: {} px fits too", fit_size(j));
        }
        if !fits(bl, aw, ah, fit.px, TIGHT_STEPS) {
            assert_eq!(fit, Fit { px: ui_small_size(), tight: 0 }, "{what}");
            return;
        }
        assert!(fits(bl, aw, ah, fit.px, fit.tight), "{what}");
        assert!(fit.tight == 0 || !fits(bl, aw, ah, fit.px, fit.tight - 1), "{what}: a step more open fits too");
    }

    /// A sheet r on a screen ah high starts below the Library's top bar's buttons or at the margin
    /// above them, as the tallest sheet does, and ends above the Library's pager or at the margin
    /// under it.
    fn clear_of_bars(ah: i32, r: Rect, what: &str) {
        let end = r.y + r.h;
        assert!(r.y >= crate::library::top_buttons_foot() || r.y == ui_px(40.0), "{what}: starts at {}", r.y);
        assert!(end <= pager_top(ah) || end == ah - ui_px(40.0), "{what}: ends at {end}");
    }

    #[test]
    fn every_note_fits_one_page_upright_and_sideways() {
        // The owner, 2026-11-07: "The read all doesn't need text that big -- either smaller text for
        // all past messages, or fit all (which i lean toward)", and "The note reading itself isn't
        // polished enough". Every version's notes in docs/whatsnew/ are whole on one page, upright and
        // sideways on the Scribe, in both looks, set as note_fit sets them (check_fit), and none needs
        // the floor's pages. No page arrows, no "1 / 2". Classic in each of its fonts (2.1, Settings >
        // Font), one after another as a pick in Settings changes it: each font's size is its own.
        let mut looks = vec![(UiTheme::Gothic, 0)];
        looks.extend((0..FONT_FAMILIES.len()).map(|f| (UiTheme::Classic, f)));
        for (theme, family) in looks {
            up(theme);
            font_set_family(family);
            let font = FONT_FAMILIES[family].id;
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                for e in notes() {
                    let mut c = screen(aw, ah);
                    let h = draw_note(&mut c, aw, ah, e, 0);
                    let bl = about_blocks(e.md);
                    let fit = note_fit(&bl, aw, ah);
                    let what = format!("{theme:?} {font} {aw}x{ah} {}", e.label());
                    let need = rows_h(&rows(&bl, note_w(aw), &note_look(fit.px, fit.tight)));
                    let at44 = |t: i32| rows_h(&rows(&bl, note_w(aw), &note_look(ui_text_size(), t)));
                    eprintln!(
                        "{what}: body {} px (type {} px high), leading {:.2}, {} page(s), {need} of {} px (44 px: {} open, {} tightest), sheet {} to {}",
                        h.body_px,
                        type_h(h.body_px),
                        h.lead,
                        h.pages,
                        body_max(ah),
                        at44(0),
                        at44(TIGHT_STEPS),
                        h.area.y,
                        h.area.y + h.area.h
                    );
                    assert_eq!((h.body_px, h.lead), (fit.px, note_look(fit.px, fit.tight).lead), "{what}");
                    check_fit(&bl, aw, ah, fit, &what);
                    assert_eq!(h.pages, 1, "{what}");
                    assert!(h.signed && fits(&bl, aw, ah, fit.px, fit.tight));
                    assert!(rect_is_empty(h.prev) && rect_is_empty(h.next));
                    assert!(inside(Rect::new(0, 0, aw, ah), h.area) && inside(h.area, h.back) && inside(h.area, h.close));
                    clear_of_bars(ah, h.area, &what);
                }
            }
        }
        font_set_family(0);
    }

    #[test]
    fn a_note_tightens_its_leading_and_gaps_before_its_type_gets_smaller() {
        // The owner, 2026-11-07: "The note reading itself isn't polished enough". Notes too long for
        // one screen in the look Read all has always had are first set closer at the size they have
        // always had (the leading down to LEAD_MIN, the gaps to GAPS_MIN of theirs); only past that is
        // their type smaller, and the room the smaller type leaves goes back to the leading and the
        // gaps, up to the open look. (Fitting by size alone set 2.0.0's notes in Gothic in its smaller
        // type over 1095 of 1940 px, upright.)
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                let what = format!("{theme:?} {aw}x{ah}");
                // A short note keeps the size and the look Read all has always had.
                let short = note("# Short\n\n- one change\n\n{signature}\n".into());
                let mut c = screen(aw, ah);
                let h = draw_note(&mut c, aw, ah, &short, 0);
                assert_eq!((h.body_px, h.lead, h.pages), (ui_text_size(), LEAD, 1));
                let k = note_look(h.body_px, 0);
                assert_eq!((k.body, k.head_px, k.lead), (ui_text_size(), ui_text_size(), 1.3));
                assert_eq!((k.indent, k.head_gap, k.item_gap, k.para_gap, k.sign_w, k.sign_h), (ui_px(56.0), ui_px(44.0), ui_px(8.0), ui_px(24.0), ui_px(720.0), ui_px(190.0)));
                // The tightest look at that size: the leading 1.15, the gaps 60% of theirs, the rest
                // as it was.
                let t = note_look(ui_text_size(), TIGHT_STEPS);
                assert!((t.lead - 1.15).abs() < 1e-4 && t.body == ui_text_size() && t.dot == k.dot);
                assert_eq!((t.indent, t.head_gap, t.item_gap, t.para_gap, t.sign_w, t.sign_h), (ui_px(56.0), ui_px(26.4), ui_px(4.8), ui_px(14.4), ui_px(720.0), ui_px(190.0)));
                let md = |n: usize| (0..n).map(|i| format!("- change {i}, in words enough for a line\n")).collect::<String>() + "\n{signature}\n";
                // Bullets enough that the open look takes two pages and the tightest holds them: the
                // size they always had, set as little closer as fits.
                let n = (1..).find(|&n| !fits(&about_blocks(&md(n)), aw, ah, ui_text_size(), 0)).unwrap();
                let bl = about_blocks(&md(n));
                assert!(fits(&bl, aw, ah, ui_text_size(), TIGHT_STEPS), "{what}: {n} bullets");
                let h = draw_note(&mut c, aw, ah, &note(md(n)), 0);
                let fit = note_fit(&bl, aw, ah);
                eprintln!("{what}: {n} bullets at {} px, leading {:.2}", h.body_px, h.lead);
                assert_eq!((h.body_px, h.pages), (ui_text_size(), 1));
                assert!(fit.tight > 0 && h.lead < LEAD && h.lead > LEAD_MIN - 1e-4);
                check_fit(&bl, aw, ah, fit, &what);
                // More, so that even the tightest look takes two pages at that size: a smaller size,
                // the largest at which the tightest fits, as open as the room it leaves allows.
                let n = (n..).find(|&n| !fits(&about_blocks(&md(n)), aw, ah, ui_text_size(), TIGHT_STEPS)).unwrap();
                let bl = about_blocks(&md(n));
                assert!(fits(&bl, aw, ah, ui_small_size(), TIGHT_STEPS), "{what}: {n} bullets");
                let h = draw_note(&mut c, aw, ah, &note(md(n)), 0);
                let fit = note_fit(&bl, aw, ah);
                eprintln!("{what}: {n} bullets at {} px, leading {:.2}", h.body_px, h.lead);
                assert!(h.body_px < ui_text_size() && h.body_px >= ui_small_size() && h.pages == 1);
                check_fit(&bl, aw, ah, fit, &what);
                let (k, k0) = (note_look(fit.px, fit.tight), note_look(ui_text_size(), 0));
                assert!(k.indent <= k0.indent && k.sign_h < k0.sign_h);
                if theme == UiTheme::Gothic {
                    // Gothic's type has two sizes here: the first step that changes it (to 43 px, the
                    // card's type) makes it two thirds as tall, and its room goes back to the open look.
                    let i = (0..=FIT_STEPS).position(|i| fit_size(i) == fit.px).unwrap() as i32;
                    assert_eq!((type_h(fit.px), fit.tight), (type_h(ui_small_size()), 0), "{what}");
                    assert!((0..i).all(|j| type_h(fit_size(j)) == type_h(ui_text_size())), "{what}");
                }
            }
        }
    }

    #[test]
    fn a_note_too_long_for_the_floor_takes_pages_at_the_floor() {
        // The floor is ui_small_size, the card's size for the same notes: a note that does not fit
        // there even at the tightest is set at it (no smaller), in the open look, and takes pages, the
        // signature on the last, the arrows and "1 / N" between Back and Dismiss.
        let long: String = (0..90).map(|i| format!("- change {i}, which takes a line or two to say at any of these sizes\n")).collect();
        let e = note(format!("# Long\n\n{long}\nCheers,\n\n{{signature}}\n"));
        let bl = about_blocks(e.md);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                assert!(!fits(&bl, aw, ah, ui_small_size(), TIGHT_STEPS));
                let mut c = screen(aw, ah);
                let first = draw_note(&mut c, aw, ah, &e, 0);
                let last = draw_note(&mut c, aw, ah, &e, first.pages - 1);
                eprintln!("{theme:?} {aw}x{ah}: the long note takes {} pages at {} px", first.pages, first.body_px);
                assert_eq!((first.body_px, last.body_px, first.lead), (ui_small_size(), ui_small_size(), LEAD));
                assert!(first.pages > 1 && !first.signed && last.signed);
                assert!(!rect_is_empty(first.next) && first.back.x + first.back.w <= first.prev.x && first.next.x + first.next.w <= first.close.x);
                // the tallest sheet, each page's rows in its body, above the foot
                assert_eq!(first.area.h, ah - 2 * ui_px(40.0));
                let pages = paginate(&rows(&bl, note_w(aw), &note_look(ui_small_size(), 0)), body_max(ah));
                assert_eq!(pages.len() as i32, first.pages);
                assert!(pages.iter().all(|p| items_h(p) <= body_max(ah)));
            }
        }
    }

    #[test]
    fn a_sheet_never_cuts_across_the_top_bar_or_the_pager() {
        // Every height a note's body can take: the sheet starts as sheet_rect places it when that is
        // below the Library's top bar's buttons (under the bar, or a little higher, the bat in sight),
        // or at the margin above them, as the tallest sheet does (sheet_rect started a sheet too tall
        // for that across the buttons, their icons' tops cut off above its edge: 2.0.0's notes
        // sideways in Classic); and it ends above the Library's pager, or covers the pager down to the
        // margin, as the tallest sheet does.
        let foot = crate::library::top_buttons_foot();
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                let (mut raised, mut kept, mut grown) = (0, 0, 0);
                for body_h in (0..=body_max(ah)).step_by(5) {
                    let (r, was) = (sheet_place(aw, ah, body_h), sheet_rect(aw, ah, body_h));
                    clear_of_bars(ah, r, &format!("{theme:?} {aw}x{ah} {body_h}"));
                    assert!((r.x, r.w) == (was.x, was.w) && r.h >= was.h);
                    assert!(r.y == was.y || (was.y < foot && r.y == ui_px(40.0)));
                    assert!(r.h == was.h || (r.y + was.h > pager_top(ah) && r.y + r.h == ah - ui_px(40.0)));
                    raised += (r.y < was.y) as i32;
                    kept += (r.y == was.y && (foot..sheet_top()).contains(&r.y)) as i32;
                    grown += (r.h > was.h) as i32;
                }
                // (some sheets start between the buttons' foot and the bar's rule: they stay there)
                assert!(raised > 0 && kept > 0 && grown > 0, "{theme:?} {aw}x{ah}: {raised} {kept} {grown}");
                // Read all's list the same, with a row for each of 1 to 40 versions (sheet_rect's list
                // of 6 sideways ended over the pager, of 9 upright too).
                for n in 1..=40 {
                    clear_of_bars(ah, list_layout(aw, ah, n).card, &format!("{theme:?} {aw}x{ah} list of {n}"));
                }
            }
        }
    }

    #[test]
    fn the_sheet_has_the_body_it_was_measured_for_at_any_dpi() {
        // At a dpi other than 300 ui_px rounds each measure on its own. What the sheet's edges and
        // title take is ui_sheet's own (ui_sheet_inner), so the body note_fit fits the notes to is
        // the body ui_sheet gives them, and every note is whole on one page there too. (Two sums of
        // their own gave Gothic at 320 dpi 1 px less than measured: every note took two pages, the
        // second holding its last row.)
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            for (dpi, w, h) in [(320.0, 1986, 2648), (250.0, 1550, 2067), (212.0, 1072, 1448), (167.0, 600, 800)] {
                ui_init(dpi);
                assert_eq!(font_init(), 0);
                ui_set_theme(theme);
                for (aw, ah) in [(w, h), (h, w)] {
                    let what = format!("{theme:?} {dpi} dpi {aw}x{ah}");
                    let mut c = screen(aw, ah);
                    for body_h in [0, body_max(ah) / 2, body_max(ah)] {
                        let card = sheet_place(aw, ah, body_h);
                        let inner = ui_sheet(&mut c, card, "What's new");
                        assert_eq!((card.w - inner.w, card.h - inner.h), sheet_chrome(), "{what}");
                        assert!(sheet_body(inner).h >= body_h && sheet_body(inner).h == card.h - sheet_fixed(), "{what}");
                    }
                    for e in notes() {
                        let hits = draw_note(&mut c, aw, ah, e, 0);
                        let fit = note_fit(&about_blocks(e.md), aw, ah);
                        eprintln!("{what} {}: {} px, leading {:.2}, {} page(s)", e.label(), hits.body_px, hits.lead, hits.pages);
                        // one page whenever the notes fit at all (the smallest screens' do not)
                        assert!(hits.pages == 1 || !fits(&about_blocks(e.md), aw, ah, ui_small_size(), TIGHT_STEPS), "{what} {}", e.label());
                        assert!(dpi < 300.0 || hits.pages == 1, "{what} {}", e.label());
                        assert_eq!(hits.body_px, fit.px);
                    }
                }
            }
        }
        ui_init(300.0);
    }

    #[test]
    fn a_bullets_dot_keeps_the_cards_proportion_to_its_text() {
        // The card's dot beside its text (ui_small_size) is the measure: a note at the card's size has
        // the card's dots, at another size dots in proportion to its type as it is drawn, however
        // tight. (Shrunk from the 44 px look's, where the card's dot stood beside larger type, a note
        // at the floor had dots 0.77 of the card's beside the same type.)
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for tight in [0, TIGHT_STEPS] {
                assert_eq!(note_look(ui_small_size(), tight).dot, 1.0);
                for i in 0..=FIT_STEPS {
                    let px = fit_size(i);
                    let k = note_look(px, tight).dot;
                    assert!((k - type_h(px) as f32 / type_h(ui_small_size()) as f32).abs() < 1e-6 && k >= 1.0, "{theme:?} {px}");
                }
            }
            let big = note_look(ui_text_size(), 0).dot;
            eprintln!("{theme:?}: a 44 px note's dot is {big} times the card's");
            if theme == UiTheme::Gothic {
                // Gothic's type from 43 px down is the card's, and so are its dots; at 44 px (and
                // 43.5) its type is half again as tall, and so are its dots.
                assert_eq!((note_look(43.0, 0).dot, note_look(43.5, 0).dot, big), (1.0, 1.5, 1.5));
            } else {
                assert!((big - 44.0 / 34.0).abs() < 0.05);
            }
        }
    }

    #[test]
    fn a_notes_size_is_found_once_for_each_screen() {
        // note_fit runs once for a note, an orientation, a look and a font, not at each draw (a page
        // turned, a step back to the notes).
        up(UiTheme::Gothic);
        let e = note("# Once\n\n- a change\n\n{signature}\n".into());
        let mut c = screen(1860, 2480);
        let runs = FIT_RUNS.get();
        for page in [0, 1, 0] {
            draw_note(&mut c, 1860, 2480, &e, page);
        }
        assert_eq!(FIT_RUNS.get(), runs + 1);
        let mut s = screen(2480, 1860);
        draw_note(&mut s, 2480, 1860, &e, 0);
        draw_note(&mut s, 2480, 1860, &e, 0);
        draw_note(&mut c, 1860, 2480, &e, 0);
        assert_eq!(FIT_RUNS.get(), runs + 2);
        up(UiTheme::Classic);
        draw_note(&mut c, 1860, 2480, &e, 0);
        draw_note(&mut c, 1860, 2480, &e, 0);
        assert_eq!(FIT_RUNS.get(), runs + 3);
        // Another of Classic's fonts (2.1) is found anew, once; back in the first, its size is known.
        font_set_family(FONT_FAMILIES.len() - 1);
        draw_note(&mut c, 1860, 2480, &e, 0);
        draw_note(&mut c, 1860, 2480, &e, 0);
        assert_eq!(FIT_RUNS.get(), runs + 4);
        font_set_family(0);
        draw_note(&mut c, 1860, 2480, &e, 0);
        assert_eq!(FIT_RUNS.get(), runs + 4);
        // Gothic has one face whatever Classic's font: no search for the font picked.
        up(UiTheme::Gothic);
        font_set_family(1);
        draw_note(&mut c, 1860, 2480, &e, 0);
        assert_eq!(FIT_RUNS.get(), runs + 4);
    }

    /// What the card shows, for the test's output: each block's first words, and whether it is cut.
    fn shown(l: &Card, bl: &[Block]) -> String {
        let notes: Vec<Block> = highlights(bl).into_iter().filter(|b| *b != Block::Signature).collect();
        let mut out = String::new();
        for b in notes.iter().take(l.blocks) {
            let t = match b {
                Block::Heading(t) | Block::Bullet(t) | Block::Para(t) => t.as_str(),
                _ => "",
            };
            out.push_str(&format!("  {}\n", t.split_whitespace().take(6).collect::<Vec<_>>().join(" ")));
        }
        out
    }

    #[test]
    fn the_card_ends_at_the_last_block_that_fits_whole() {
        // The owner's ruling of 2026-11-05 ("do the small change"): 2.0.0's notes cut the Ko-fi
        // paragraph mid-sentence above the signature. Upright and sideways on the Scribe, in both
        // looks, the card ends with a whole block, and the next block would not have fitted. The
        // card's notes are this version's.
        let bl = about_blocks(notes()[current(notes(), GRIMOIRE_VERSION)].md);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                let l = card_layout(aw, ah, bat(), &bl);
                eprintln!("{theme:?} {aw}x{ah}: {} of {} blocks, room {}\n{}", l.blocks, l.notes, l.room, shown(&l, &bl));
                assert!(l.n > 0 && !l.cut, "{theme:?} {aw}x{ah}");
                // The card ends above the Library's pager (sideways it covered the pager's left button
                // and the P of Page 1 of 1 in Gothic).
                let h = draw_card(&mut screen(aw, ah), aw, ah, bat(), &bl);
                assert!(h.area.y + h.area.h <= pager_top(ah), "{theme:?} {aw}x{ah}: the card ends at {}", h.area.y + h.area.h);
                assert!(l.rs[l.n - 1].end);
                assert!(rows_h(&l.rs[..l.n]) <= l.room);
                if l.n < l.rs.len() {
                    // (the block after the last one shown, a heading with the block under it, is
                    // taller than what is left)
                    let next_end = (l.n..l.rs.len()).find(|&i| l.rs[i].end).unwrap();
                    assert!(rows_h(&l.rs[..next_end + 1]) > l.room);
                    assert!(l.blocks < l.notes);
                } else {
                    assert_eq!(l.blocks, l.notes);
                }
            }
        }
    }

    #[test]
    fn a_first_paragraph_too_long_for_the_card_is_cut_at_a_line() {
        up(UiTheme::Gothic);
        let long = vec!["words that go on and on"; 400].join(" ");
        let md = format!("# T\n\n{long}\n\n- after\n\n{{signature}}\n");
        let bl = about_blocks(&md);
        let l = card_layout(1860, 2480, bat(), &bl);
        let para = l.rs.iter().filter(|r| r.block == 0).count();
        assert!(l.cut && l.n > 0 && l.n < para, "{} of {para} lines", l.n);
        assert!(l.rs[..l.n].iter().all(|r| r.block == 0));
        assert!(rows_h(&l.rs[..l.n]) <= l.room);
        // (the next line would not have fitted: as many lines as there is room for)
        assert!(rows_h(&l.rs[..l.n + 1]) > l.room);
        assert_eq!(l.blocks, 1);
        // Whole lines: each row is one of the paragraph's wrapped lines, as Read all has them.
        let lines = wrap(FontFace::Regular, l.k.body, &long, l.iw);
        assert_eq!(lines.len(), para);
        // A paragraph that fits but is followed by one that does not: the card ends after the first.
        let md = format!("# T\n\nShort.\n\n{long}\n\n{{signature}}\n");
        let l = card_layout(1860, 2480, bat(), &about_blocks(&md));
        assert!(!l.cut && l.blocks == 1 && l.n == 1);
    }

    #[test]
    fn a_word_wider_than_the_sheet_is_broken_not_cut() {
        // A link longer than a line, and Japanese, which has no spaces: wrap breaks the link between
        // its characters on lines of its own, and Japanese between any two of its characters (never
        // before a closing mark such as 。 or 」, nor after an opening one such as 「), so every line
        // fits whole and nothing is cut short with "...". (wrap broke only at spaces: each was one
        // line, drawn cut at the sheet's edge at every size, which the fit, by height, never saw.)
        let url = format!("https://example.com/{}", "grimoire-notes/".repeat(12));
        let link = format!("See {url} for the rest.");
        let ja = "グリモアはキンドルスクライブのためのノートです。「すべて読む」を押すと、すべての版のお知らせが一覧になり、タップするとその版のお知らせが一画面に収まります。".repeat(3);
        let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for aw in [1860, 2480] {
                let w = note_w(aw);
                for px in [ui_text_size(), ui_small_size()] {
                    for (text, iw) in [(&link, w - note_look(px, 0).indent), (&ja, w)] {
                        let lines = wrap(FontFace::Regular, px, text, iw);
                        assert!(lines.len() > 1, "{theme:?} {aw} {px}: {lines:?}");
                        for l in &lines {
                            assert!(font_measure(FontFace::Regular, px, l) <= iw, "{theme:?} {aw} {px}: {l}");
                            assert!(!NO_BREAK_BEFORE.contains(l.chars().next().unwrap()) && !NO_BREAK_AFTER.contains(l.chars().last().unwrap()), "{l}");
                        }
                        // nothing lost or added: the lines are the text, the spaces at their breaks aside
                        assert_eq!(strip(&lines.concat()), strip(text));
                    }
                    // The link starts a line of its own, and its lines are full ones but the last.
                    let lines = wrap(FontFace::Regular, px, &link, w);
                    assert_eq!(lines[0], "See");
                    assert!(lines[1].starts_with("https://"));
                }
            }
            // Words between spaces stay whole, as before; Latin beside Japanese may break.
            assert_eq!(wrap(FontFace::Regular, 44.0, "one two  three", 10000), ["one two three"]);
            let two = wrap(FontFace::Regular, 44.0, "one two three", font_measure(FontFace::Regular, 44.0, "one two"));
            assert_eq!(two, ["one two", "three"]);
            assert_eq!(pieces("Kindleの「本」。"), [("Kindle".into(), true), ("の".into(), false), ("「本」。".into(), false)]);
        }
    }

    #[test]
    fn signature_pgm() {
        assert_eq!(pgm(b"P5\n3 2\n255\n\x00\x80\xff\x10\x20\x30"), Some((3, 2, &b"\x00\x80\xff\x10\x20\x30"[..])));
        assert_eq!(pgm(b"P5 3 2 255 \x00\x80\xff\x10\x20"), None); // (short)
        assert_eq!(pgm(b"P5\n3 2\n65535\n"), None);
        assert_eq!(pgm(b"P6\n1 1\n255\n\x00"), None);
        // The built-in signature reads, and keeps its shape in its place.
        let (w, h, px) = pgm(SIGNATURE).expect("assets/signature.pgm");
        assert_eq!(px.len(), (w * h) as usize);
        let (bw, bh) = sig_box(560, 130);
        assert!(bw <= 560 && bh <= 130 && (bw as f32 / bh as f32 - w as f32 / h as f32).abs() < 0.1);
    }
}
