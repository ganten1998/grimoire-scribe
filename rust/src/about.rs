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
//! one. A tap on a row shows that version's notes whole, a page at a time, with Back to the list;
//! Dismiss closes What's new from any of them (2.0.1).
//!
//! The notes, a file for each version (2.0.1): build.rs lists docs/whatsnew/*.md, each named for its
//! version (VERSION.md), with the date CHANGELOG.md's "## VERSION (YYYY-MM-DD)" heading gives it, into
//! NOTE_FILES; a new release's notes are a new file there, and no code changes. The archive (notes)
//! puts them newest first by version (update_version_cmp: 2.0.1, 2.0.0, 2.0.0-rc.2, 2.0.0-beta.2,
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
    ARCHIVE.get_or_init(|| archive(NOTE_FILES))
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

/// Lines of text broken at spaces to fit w (a word wider than w alone keeps its own line).
fn wrap(f: FontFace, px: f32, text: &str, w: i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if line.is_empty() || font_measure(f, px, &trial) <= w {
            line = trial;
        } else {
            out.push(std::mem::replace(&mut line, word.to_string()));
        }
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
    Dot,
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
                        items.push(Item { r: Rect::new(0, 0, k.indent, body_lh), ink: Ink::Dot });
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
            Ink::Dot => {
                let (cx, cy) = (r.x as f32 + r.w as f32 * 0.4, r.y as f32 + r.h as f32 / 2.0);
                if gothic() {
                    canvas_lozenge(c, cx, cy, ui_px(10.0) as f32, 0);
                } else {
                    canvas_disc_aa(c, cx, cy, ui_px(8.0) as f32, 0, 1.0);
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
    };
    let head_h = ui_px(84.0);
    let btn_h = ui_px(110.0);
    let top = bat.y + bat.h + ui_px(44.0);
    let fixed = 2 * pad + head_h + ui_px(10.0) + ui_px(30.0) + btn_h;
    // The highlights the screen has room for, each whole (Read all has the rest), the signature always.
    let short = highlights(bl);
    let sig = short.last() == Some(&Block::Signature);
    let notes = &short[..short.len() - sig as usize];
    let (sw, sh) = sig_box(k.sign_w, k.sign_h);
    let sig_h = if sig { k.para_gap + sh } else { 0 };
    let room = (ah - ui_px(40.0) - top - fixed - sig_h).max(0);
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

/// What the sheet's edges and title take (ui_sheet's), across and down.
fn sheet_chrome() -> (i32, i32) {
    if gothic() {
        (ui_px(80.0), ui_px(210.0))
    } else {
        (ui_px(72.0), ui_px(192.0))
    }
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
/// bat in sight, when the screen has the room.
fn sheet_rect(aw: i32, ah: i32, body_h: i32) -> Rect {
    let w = sheet_w(aw);
    let h = (sheet_fixed() + body_h).min(ah - 2 * ui_px(40.0));
    Rect::new((aw - w) / 2, (ah - ui_px(40.0) - h).min(ui_px(214.0)).max(ui_px(40.0)), w, h)
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
/// turn keeps it where it is.
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
    let card = sheet_rect(aw, ah, shown * (rh + gap) - gap);
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

/// A version's notes whole on the Library's sheet: their title as its title (else What's new), the
/// bat, the version and its date under it, the notes a page at a time in the larger type, Back to the
/// list at the foot's left and Dismiss at its right (the page arrows left of Dismiss).
fn draw_note(c: &mut Canvas, aw: i32, ah: i32, e: &Entry, page: i32) -> AboutHits {
    let bl = about_blocks(e.md);
    let k = Look {
        body: ui_text_size(),
        head_f: FontFace::Bold,
        head_px: ui_text_size(),
        lead: 1.3,
        indent: ui_px(56.0),
        head_gap: ui_px(44.0),
        item_gap: ui_px(8.0),
        para_gap: ui_px(24.0),
        sign_w: ui_px(720.0),
        sign_h: ui_px(190.0),
    };
    let rs = rows(&bl, sheet_w(aw) - sheet_chrome().0, &k);
    let card = sheet_rect(aw, ah, rows_h(&rs));
    let inner = ui_sheet(c, card, md_title(&bl).unwrap_or(whats_new()));
    draw_head(c, inner, &e.label(), date_label(e.date));
    let body = sheet_body(inner);
    let pages = paginate(&rs, body.h);
    let mut hits = AboutHits { area: card, pages: pages.len() as i32, ..AboutHits::default() };
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
        // blank line more), rc.10 2.0.1's.
        let files = [
            file("2.0.0-beta.1", "2026-11-01", "Dear reader,\n\n- one\n- two\n\n{signature}\n"),
            file("2.0.0-rc.2", "", "# Since rc.1\n\n- a fix\n"),
            file("2.0.1", "unreleased", "# What's new (since 2.0.0)\n\n- wake\n\n{signature}\n"),
            file("2.0.0-beta.10", "", "# Ten\n\n- ten\n"),
            file("2.0.0", "2026-11-05", "# What's new (since 2.0.0-beta.1)\n\n- lots\n\n{signature}\n"),
            file("2.0.0-beta.2", "2026-11-05", "Dear\nreader,\n\n\n- one\n- two\n{signature}"),
            file("2.0.0-rc.10", "", "# What's new (since 2.0.0)\n- wake\n\n{signature}"),
        ];
        let a = archive(&files);
        let got: Vec<String> = a.iter().map(|e| e.label()).collect();
        // (by version, numbers as numbers: rc.10 after 2.0.1's place, beta.10 before beta.2)
        assert_eq!(got, ["2.0.1, 2.0.0-rc.10", "2.0.0", "2.0.0-rc.2", "2.0.0-beta.10", "2.0.0-beta.2, 2.0.0-beta.1"]);
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
        // A version with no notes of its own (2.0.1 before its file is written) shows the newest.
        assert_eq!(current(&a, "2.0.1"), 0);
        assert_eq!(a[current(&a, "2.0.1")].versions[0], "2.0.0");
        assert_eq!(current(&a, "1.2.0"), 0);
        // The built-in notes: this version has its own (docs/RELEASING.md, The bump commit), and the
        // card shows them.
        assert!(notes()[current(notes(), GRIMOIRE_VERSION)].versions.contains(&GRIMOIRE_VERSION));
    }

    #[test]
    fn the_built_in_notes_are_the_folders() {
        // build.rs's list is docs/whatsnew/*.md, every file, with its text (an editor's files passed
        // over as build.rs passes them: .#2.0.1.md, #2.0.1.md#, 2.0.2.md~).
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

    /// n entries, 2.0.n-1 down to 2.0.0, each its own note.
    fn entries(n: usize) -> Vec<Entry> {
        (0..n)
            .rev()
            .map(|i| Entry {
                versions: vec![Box::leak(format!("2.0.{i}").into_boxed_str())],
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
                        let h = draw_list(&mut c, aw, ah, &all, "2.0.1", page);
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
                        // 2.0.1 (the entry before the oldest) marked in its row, on its page alone
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
