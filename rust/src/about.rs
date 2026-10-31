//! The About sheet, "What's new": the notes in docs/WHATSNEW.md (built in) and the owner's
//! signature. The bat in the Library's top bar opens it (library.rs), and so does the first start
//! after an update unless "Don't show this again" was ticked. Rust only (2.0.0).
//!
//! It opens as a card hanging from the bat: What's new, the notes (each heading with its first line;
//! notes without headings, as much of them as the card has room for), the signature, the box, Read
//! all and Close. Read all shows the whole notes on the Library's sheet, a page at a time when they
//! need more than one.
//!
//! WHATSNEW.md is a small Markdown: "# " the title, "## " a heading, "- " a bullet, other lines a
//! paragraph (lines joined until a blank one), and a line "{signature}" where the signature goes.
//! It is the owner's text and is not translated; the sheet's own words are.

use crate::prelude::*;

pub static WHATSNEW: &str = include_str!("../../docs/WHATSNEW.md");

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

/// WHATSNEW.md's blocks, in order.
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
    for b in bl {
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
                    out.push(Row { h: body_lh, gap: if i == 0 { k.item_gap } else { 0 }, keep_next: false, items });
                }
            }
            Block::Para(t) => {
                for (i, l) in wrap(FontFace::Regular, k.body, t, w).into_iter().enumerate() {
                    out.push(Row {
                        h: body_lh,
                        gap: if i == 0 { k.para_gap } else { 0 },
                        keep_next: false,
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
                out.push(Row { h: sh, gap: k.para_gap, keep_next: false, items: vec![Item { r: Rect::new(0, 0, sw, sh), ink: Ink::Sign }] });
            }
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

/// Where the sheet's controls are (empty when not shown), and its pages.
#[derive(Clone, Copy, Debug, Default)]
pub struct AboutHits {
    pub close: Rect,
    pub hide: Rect,
    pub prev: Rect,
    pub next: Rect,
    pub more: Rect,
    pub pages: i32,
}

/// Draws What's new: the card hanging from the bat (bat: its rect in the top bar), or with full
/// (Read all) the whole notes on the sheet, at page (from 0; kept in range). hide: the box ticked.
pub fn about_draw(c: &mut Canvas, aw: i32, ah: i32, bat: Rect, page: i32, full: bool, hide: bool) -> AboutHits {
    let bl = about_blocks(WHATSNEW);
    if full {
        draw_sheet(c, aw, ah, &bl, page, hide)
    } else {
        draw_card(c, aw, ah, bat, &bl, hide)
    }
}

fn hide_label() -> &'static str {
    // TRANSLATORS: the What's new card's box (1040 px at 44 px; on the sheet, what Close leaves):
    // when ticked, What's new no longer opens by itself after an update (the bat in the Library's
    // top bar still opens it).
    tr("Don't show this again")
}

fn close_label() -> &'static str {
    // TRANSLATORS: What's new's button that closes it (a word under an icon, at 34 px: 200 px).
    tr("Close")
}

/// The card: What's new and the version, the highlights and the signature, the box, Read all and
/// Close. It hangs from the bat, a notch pointing up at it.
fn draw_card(c: &mut Canvas, aw: i32, ah: i32, bat: Rect, bl: &[Block], hide: bool) -> AboutHits {
    let mut hits = AboutHits { pages: 1, ..AboutHits::default() };
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
    let box_h = ui_px(110.0);
    let btn_h = ui_px(110.0);
    let top = bat.y + bat.h + ui_px(44.0);
    let fixed = 2 * pad + head_h + ui_px(10.0) + ui_px(30.0) + box_h + ui_px(14.0) + btn_h;
    // The highlights the screen has room for (Read all has the rest), the signature always.
    let short = highlights(bl);
    let sig = short.last() == Some(&Block::Signature);
    let notes = &short[..short.len() - sig as usize];
    let (sw, sh) = sig_box(k.sign_w, k.sign_h);
    let sig_h = if sig { k.para_gap + sh } else { 0 };
    let room = (ah - ui_px(40.0) - top - fixed - sig_h).max(0);
    let rs = rows(notes, iw, &k);
    let page = paginate(&rs, room).swap_remove(0);
    let notes_h = items_h(&page);
    let card = Rect::new(bat.x.max(ui_px(40.0)).min(aw - w - ui_px(40.0)), top, w, fixed + notes_h + sig_h);
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
    let inner = Rect::new(card.x + pad, card.y + pad, iw, card.h - 2 * pad);
    // TRANSLATORS: the card's title (and the sheet's when the notes have none): what this version
    // brings (520 px at 44 px in bold).
    font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(inner.x, inner.y, iw * 2 / 3, head_h), tr("What's new"), 0);
    let ver = GRIMOIRE_VERSION;
    let vw = font_measure(FontFace::Regular, k.body, ver);
    font_draw_left_name(c, FontFace::Regular, k.body, Rect::new(inner.x + iw - vw, inner.y, vw, head_h), ver, 90);
    canvas_hline(c, inner.x, inner.x + iw, inner.y + head_h, ui_px(2.0), 0);
    let body = Rect::new(inner.x, inner.y + head_h + ui_px(10.0), iw, notes_h);
    draw_items(c, &page, body);
    if sig {
        draw_signature(c, Rect::new(inner.x, body.y + notes_h + k.para_gap, sw, sh));
    }
    let by = inner.y + inner.h - btn_h;
    hits.hide = Rect::new(inner.x, by - ui_px(14.0) - box_h, iw, box_h);
    ui_checkbox(c, hits.hide, hide_label(), hide);
    let cw = ui_px(260.0);
    hits.close = Rect::new(inner.x + iw - cw, by, cw, btn_h);
    ui_button(c, hits.close, IconKind::Close, close_label(), ButtonStyle::Primary);
    let mw = ui_px(330.0);
    hits.more = Rect::new(hits.close.x - ui_px(24.0) - mw, by, mw, btn_h);
    // TRANSLATORS: the What's new card's button to the whole notes (330 px at 44 px in bold).
    ui_button(c, hits.more, IconKind::None, tr("Read all"), ButtonStyle::Plain);
    hits
}

/// Read all: the Library's sheet, the notes' title as its title, the bat and the version under it,
/// the notes a page at a time, the box and Close (and the page arrows between them).
fn draw_sheet(c: &mut Canvas, aw: i32, ah: i32, bl: &[Block], page: i32, hide: bool) -> AboutHits {
    let mut hits = AboutHits::default();
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
    let w = (aw - 2 * ui_px(70.0)).min(ui_px(1800.0));
    let (chrome_w, chrome_h) = if gothic() { (ui_px(80.0), ui_px(210.0)) } else { (ui_px(72.0), ui_px(192.0)) };
    let head_h = ui_px(70.0);
    let foot_h = ui_px(120.0);
    let rs = rows(bl, w - chrome_w, &k);
    let fixed = chrome_h + head_h + ui_px(24.0) + ui_px(36.0) + foot_h;
    let h = (fixed + rows_h(&rs)).min(ah - 2 * ui_px(40.0));
    // (under the top bar, the bat in sight, when the screen has the room)
    let card = Rect::new((aw - w) / 2, (ah - ui_px(40.0) - h).min(ui_px(214.0)).max(ui_px(40.0)), w, h);
    let inner = ui_sheet(c, card, md_title(bl).unwrap_or(tr("What's new")));
    let kb = ui_px(2.0);
    draw_bat(c, inner.x, inner.y + (head_h - BAT_H * kb) / 2, kb);
    let vx = inner.x + BAT_W * kb + ui_px(24.0);
    let ver = format!("Grimoire {}", GRIMOIRE_VERSION);
    font_draw_left_name(c, FontFace::Regular, ui_small_size(), Rect::new(vx, inner.y, inner.w - (vx - inner.x), head_h), &ver, 90);
    let body = Rect::new(inner.x, inner.y + head_h + ui_px(24.0), inner.w, inner.h - head_h - ui_px(24.0) - ui_px(36.0) - foot_h);
    let pages = paginate(&rs, body.h);
    hits.pages = pages.len() as i32;
    let page = page.clamp(0, hits.pages - 1);
    draw_items(c, &pages[page as usize], body);
    // The bottom: the box at the left, Close at the right, the page arrows between them.
    let r = Rect::new(inner.x, inner.y + inner.h - foot_h, inner.w, foot_h);
    let cw = ui_px(300.0);
    hits.close = Rect::new(r.x + r.w - cw, r.y, cw, r.h);
    ui_button(c, hits.close, IconKind::Close, close_label(), ButtonStyle::Primary);
    let arrows = if hits.pages > 1 { 2 * ui_px(130.0) + ui_px(150.0) + ui_px(40.0) } else { 0 };
    hits.hide = Rect::new(r.x, r.y, r.w - cw - ui_px(40.0) - arrows, r.h);
    ui_checkbox(c, hits.hide, hide_label(), hide);
    if hits.pages > 1 {
        let (bw, tw) = (ui_px(130.0), ui_px(150.0));
        let x = hits.close.x - ui_px(40.0) - 2 * bw - tw;
        hits.prev = Rect::new(x, r.y, bw, r.h);
        hits.next = Rect::new(x + bw + tw, r.y, bw, r.h);
        let st = |on: bool| if on { ButtonStyle::Plain } else { ButtonStyle::Disabled };
        ui_button(c, hits.prev, IconKind::Prev, "", st(page > 0));
        ui_button(c, hits.next, IconKind::Next, "", st(page + 1 < hits.pages));
        font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect::new(x + bw, r.y, tw, r.h), &format!("{} / {}", page + 1, hits.pages), 0);
    }
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
        // The built-in notes have a place for the signature, at their end.
        let b = about_blocks(WHATSNEW);
        assert_eq!(b.last(), Some(&Block::Signature));
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
