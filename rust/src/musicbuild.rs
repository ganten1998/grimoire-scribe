//! musicbuild.h: the "Custom music" sheet. Stack staves (with clefs), tabs and lyric lines
//! into a system, choose how many systems go on a page, see the page as it will look.
//! A modal hosted by the editor (paper for this page) and the library (new notebook).

use crate::prelude::*;

const B_UP: i32 = 1;
const B_DOWN: i32 = 2;
const B_REMOVE: i32 = 3;
const B_ADD: i32 = 4;
const B_LESS: i32 = 5;
const B_MORE: i32 = 6;
const B_CANCEL: i32 = 7;
const B_DONE: i32 = 8;
const B_PRESET: i32 = 9;

// The order parts are offered in the "Add" grid.
const K_PARTS: [MusicPart; 12] = [
    PART_TREBLE, PART_BASS, PART_ALTO, PART_TENOR, PART_PERC, PART_STAFF, PART_LYRICS, PART_TAB4, PART_TAB5, PART_TAB6,
    PART_TAB7, PART_TAB8,
];
// Layouts to start from: the papers the pickers offered before rc.9 (Grand staff, the bass tabs),
// each as parts with its systems per page (as the fixed papers have them).
const K_PRESETS: [(&str, [MusicPart; 2], i32); 5] = [
    ("Grand staff", [PART_TREBLE, PART_BASS], 5),
    ("4-string bass", [PART_TAB4, PART_NONE], 10),
    ("5-string bass", [PART_TAB5, PART_NONE], 10),
    ("6-string bass", [PART_TAB6, PART_NONE], 9),
    ("7-string bass", [PART_TAB7, PART_NONE], 8),
];
const N_PRESETS: i32 = K_PRESETS.len() as i32;
const N_PARTS: i32 = K_PARTS.len() as i32;
const ADD_COLS: i32 = 3;
const ADD_ROWS: i32 = (N_PARTS + ADD_COLS - 1) / ADD_COLS;
// Fixed rows: heading, gap, heading, gap, heading, gap; plus title 120, padding 72, buttons 190.
const FIXED_H: i32 = 120 + 72 + 70 + 20 + 70 + 20 + 70 + 20 + 190;
// the parts, the Add grid, the presets' row, systems per page
const ROWS: i32 = MUSIC_MAX_PARTS as i32 + ADD_ROWS + 1 + 1;

fn row_h(a: &App) -> i32 {
    clampi((a.h - ui_px(80.0) - ui_px(FIXED_H as f32)) / ROWS, ui_px(80.0), ui_px(115.0))
}

fn card_rect(a: &App) -> Rect {
    let mut w = a.w - 2 * ui_px(60.0);
    if w > ui_px(1740.0) {
        w = ui_px(1740.0);
    }
    let h = ui_px(FIXED_H as f32) + ROWS * row_h(a);
    Rect { x: (a.w - w) / 2, y: (a.h - h) / 3, w, h }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MbResult {
    #[default]
    None = 0,  // MB_NONE
    Changed = 1,  // MB_CHANGED
    Cancel = 2,  // MB_CANCEL
    Done = 3,  // MB_DONE
}

#[derive(Clone, Debug, Default)]
pub struct MusicBuild {
    pub open: bool,
    pub rect: Rect,
    pub hits: HitList,
    pub m: MusicLayout,
    pub press: Press,
    /// page preview at 1/4 size, rebuilt when the layout changes
    pub prev: Canvas,
    pub prev_k: PaperKind,
    /// the page the layout is for: how many systems fit, the preview's size
    pub page_w: i32,
    pub page_h: i32,
}


// The most systems of the layout that fit on the page it is for.
fn fit(b: &MusicBuild, a: &App) -> i32 {
    music_max_systems(&b.m, b.page_h, a.dpi)
}

fn clamp_systems(b: &mut MusicBuild, a: &App) {
    let mx = fit(b, a);
    b.m.systems = clampi(b.m.systems, 1, mx);
}

/// The same for a page of page_w x page_h (an open notebook's, which may be another screen's size).
pub fn musicbuild_open_page(b: &mut MusicBuild, a: &mut App, start: PaperKind, page_w: i32, page_h: i32) {
    canvas_free(&mut b.prev);
    *b = MusicBuild { open: true, page_w, page_h, ..Default::default() };
    if !music_decode(start, &mut b.m) && !music_decode(a.settings.music_layout, &mut b.m) {
        music_decode(music_default(), &mut b.m);
    }
    clamp_systems(b, a);
    b.rect = card_rect(a);
}

/// Starts from `start` if it is a custom layout, else from the last one built (settings), for a
/// new notebook's page (the device's page size).
pub fn musicbuild_open(b: &mut MusicBuild, a: &mut App, start: PaperKind) {
    let (mut pw, mut ph) = (0, 0);
    store_page_size(&mut pw, &mut ph);
    musicbuild_open_page(b, a, start, pw, ph);
}

pub fn musicbuild_free(b: &mut MusicBuild) {
    canvas_free(&mut b.prev);
    b.open = false;
}

pub fn musicbuild_result(b: &MusicBuild) -> PaperKind {
    music_encode(&b.m)
}

// (The C takes the App for its dpi; the draw below holds a borrow of the App's screen meanwhile.)
fn preview(b: &mut MusicBuild, dpi: f32) -> Option<&Canvas> {
    let k = music_encode(&b.m);
    if !b.prev.px.is_empty() && b.prev_k == k {
        return Some(&b.prev);
    }
    canvas_free(&mut b.prev);
    let (pw, ph) = (b.page_w, b.page_h);
    let mut full = Canvas::default();
    if canvas_init(&mut full, pw, ph) != 0 {
        return None;
    }
    paper_render(&mut full, k, Rect { x: 0, y: 0, w: pw, h: ph }, dpi);
    if canvas_init(&mut b.prev, pw / 4, ph / 4) == 0 {
        canvas_downsample_keep_lines(&mut b.prev, &full, 0.6);
    }
    canvas_free(&mut full);
    b.prev_k = k;
    if !b.prev.px.is_empty() {
        Some(&b.prev)
    } else {
        None
    }
}

/// draws into a->screen, returns its rect
pub fn musicbuild_draw(b: &mut MusicBuild, a: &mut App) -> Rect {
    hits_clear(&mut b.hits);
    b.rect = card_rect(a);
    let rh = row_h(a);
    let mx = fit(b, a);
    let dpi = a.dpi;
    let c = &mut a.screen;
    // TRANSLATORS: the title of the sheet that builds a music paper from parts.
    let in_ = ui_sheet(c, b.rect, tr("Custom music"));
    let pad = ui_px(12.0);
    let lw = in_.w * 58 / 100;
    let ts = ui_text_size();
    let mut y = in_.y;
    // TRANSLATORS: the Custom music sheet's headings, over the layout's parts and over the parts to
    // add (962 px at 44 px).
    font_draw_left(c, FontFace::Bold, ts, Rect { x: in_.x, y, w: lw, h: ui_px(70.0) }, tr("Each system, top to bottom"), 0);
    y += ui_px(70.0);
    for i in 0..MUSIC_MAX_PARTS as i32 {
        let row = Rect { x: in_.x, y: y + pad / 2, w: lw, h: rh - pad };
        if i >= b.m.n {
            if i == b.m.n {
                font_draw_left(c, FontFace::Regular, ui_small_size(), rect_inflate(row, -ui_px(20.0)), tr("(add a part below)"), 150);
            }
            y += rh; // (the C's loop step, `continue` included)
            continue;
        }
        ui_frame(c, row, ui_px(12.0), 0, false, ui_px(3.0));
        let mut lab = String::new();
        csnprintf!(&mut lab, 96, "%d  %s", i + 1, part_label(b.m.part[i as usize]));
        font_draw_left(c, FontFace::Regular, ts, Rect { x: row.x + ui_px(24.0), y: row.y, w: row.w, h: row.h }, &lab, 0);
        let bs = row.h - 2 * pad;
        let mut bx = row.x + row.w - pad - bs;
        let btn = [(B_REMOVE, IconKind::Close, b.m.n > 1), (B_DOWN, IconKind::Down, i + 1 < b.m.n), (B_UP, IconKind::Up, i > 0)];
        for (id, ic, ok) in btn {
            let br = Rect { x: bx, y: row.y + pad, w: bs, h: bs };
            ui_button(c, br, ic, "", if ok { ButtonStyle::Plain } else { ButtonStyle::Disabled });
            if ok {
                hits_add(&mut b.hits, rect_inflate(br, pad / 2), id, i);
            }
            bx -= bs + pad;
        }
        y += rh;
    }
    y += ui_px(20.0);
    font_draw_left(c, FontFace::Bold, ts, Rect { x: in_.x, y, w: lw, h: ui_px(70.0) }, tr("Add a part"), 0);
    y += ui_px(70.0);
    let full = b.m.n >= MUSIC_MAX_PARTS as i32;
    let bw = (lw - (ADD_COLS - 1) * pad) / ADD_COLS;
    // One size for every part's name, the smallest any of them needs (a longer translation takes a
    // smaller size, or two lines): a grid of sizes would read as parts of different weight.
    let mut ps = ui_text_size();
    for pt in K_PARTS {
        ps = ps.min(ui_button_text_px(Rect { x: 0, y: 0, w: bw, h: rh - pad }, part_label(pt), ButtonStyle::Plain));
    }
    for i in 0..N_PARTS {
        let r = Rect { x: in_.x + (i % ADD_COLS) * (bw + pad), y: y + (i / ADD_COLS) * rh + pad / 2, w: bw, h: rh - pad };
        ui_button_text_at(c, r, part_label(K_PARTS[i as usize]), if full { ButtonStyle::Disabled } else { ButtonStyle::Plain }, ps);
        if !full {
            hits_add(&mut b.hits, r, B_ADD, K_PARTS[i as usize].0);
        }
    }
    y += ADD_ROWS * rh + ui_px(20.0);
    // TRANSLATORS: the Custom music sheet's heading over ready-made layouts to start from (962 px at 44
    // px); under it, the layouts' names are the papers' own (Grand staff, 4-string bass...).
    font_draw_left(c, FontFace::Bold, ts, Rect { x: in_.x, y, w: lw, h: ui_px(70.0) }, tr("Start from"), 0);
    y += ui_px(70.0);
    let pw = (lw - (N_PRESETS - 1) * pad) / N_PRESETS;
    let mut qs = ui_text_size();
    for (name, _, _) in K_PRESETS {
        qs = qs.min(ui_button_text_px(Rect { x: 0, y: 0, w: pw, h: rh - pad }, c_("paper", name), ButtonStyle::Plain));
    }
    for i in 0..N_PRESETS {
        let r = Rect { x: in_.x + i * (pw + pad), y: y + pad / 2, w: pw, h: rh - pad };
        ui_button_text_at(c, r, c_("paper", K_PRESETS[i as usize].0), ButtonStyle::Plain, qs);
        hits_add(&mut b.hits, r, B_PRESET, i);
    }
    y += rh + ui_px(20.0);
    // Systems per page: [-] N [+]
    let srow = Rect { x: in_.x, y, w: lw, h: rh };
    // TRANSLATORS: left of the systems' count and its - and + buttons (481 px at 44 px).
    font_draw_left(c, FontFace::Bold, ts, Rect { x: srow.x, y: srow.y, w: lw / 2, h: rh }, tr("Systems per page"), 0);
    let bs = rh - pad;
    let less = Rect { x: in_.x + lw / 2, y: y + pad / 2, w: bs, h: bs };
    let more = Rect { x: in_.x + lw - bs, y: y + pad / 2, w: bs, h: bs };
    ui_button(c, less, IconKind::Minus, "", if b.m.systems > 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    ui_button(c, more, IconKind::Plus, "", if b.m.systems < mx { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if b.m.systems > 1 {
        hits_add(&mut b.hits, less, B_LESS, 0);
    }
    if b.m.systems < mx {
        hits_add(&mut b.hits, more, B_MORE, 0);
    }
    let mut val = String::new();
    // TRANSLATORS: the systems per page, of the most that fit (275 px at 44 px).
    csnprintf!(&mut val, 32, tr("%d of %d"), b.m.systems, mx);
    font_draw_centered(c, FontFace::Bold, ts, Rect { x: less.x + bs, y, w: more.x - less.x - bs, h: rh }, &val, 0);
    // The page as it will look.
    let mut pv = Rect { x: in_.x + lw + ui_px(50.0), y: in_.y, w: in_.w - lw - ui_px(50.0), h: in_.h - ui_px(190.0) };
    if pv.h * 3 / 4 < pv.w {
        pv.x += (pv.w - pv.h * 3 / 4) / 2;
        pv.w = pv.h * 3 / 4;
    } else {
        pv.h = pv.w * 4 / 3;
    }
    canvas_fill(c, pv, 255);
    if let Some(p) = preview(b, dpi) {
        canvas_blit_fit(c, p, pv);
    }
    ui_frame(c, rect_inflate(pv, ui_px(3.0)), ui_px(8.0), 0, false, ui_px(3.0));
    let bh = ui_px(140.0);
    let bw2 = ui_px(380.0);
    let cancel = Rect { x: in_.x, y: in_.y + in_.h - bh, w: bw2, h: bh };
    let done = Rect { x: in_.x + in_.w - bw2, y: in_.y + in_.h - bh, w: bw2, h: bh };
    ui_button(c, cancel, IconKind::None, tr("Cancel"), ButtonStyle::Plain);
    // TRANSLATORS: takes the layout built on the Custom music sheet (342 px at 34 px, under a check).
    ui_button(c, done, IconKind::Check, tr("Use layout"), ButtonStyle::Primary);
    hits_add(&mut b.hits, cancel, B_CANCEL, 0);
    hits_add(&mut b.hits, done, B_DONE, 0);
    b.rect
}

/// Pointer phase 0 down / 1 move / 2 up. MB_DONE: read musicbuild_result (already saved as
/// the last layout); MB_DONE and MB_CANCEL close the sheet (the host recomposes b->rect).
pub fn musicbuild_pointer(b: &mut MusicBuild, a: &mut App, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> MbResult {
    *dirty = RECT_EMPTY;
    if !b.open {
        return MbResult::None;
    }
    if phase == 0 {
        press_begin(&mut b.press, x, y, t);
        return MbResult::None;
    }
    press_move(&mut b.press, x, y);
    if phase != 2 {
        return MbResult::None;
    }
    if !rect_contains(b.rect, b.press.x0 as i32, b.press.y0 as i32) {
        b.open = false; // a tap outside the sheet cancels
        *dirty = b.rect;
        return MbResult::Cancel;
    }
    let Some(&h) = hits_find(&b.hits, b.press.x0, b.press.y0) else {
        return MbResult::None;
    };
    if !press_is_tap(&b.press, h.r) {
        return MbResult::None;
    }
    let i = h.arg;
    let at_max = b.m.systems >= fit(b, a); // stay "as many as fit" through edits
    match h.id {
        B_UP | B_DOWN => {
            let j = if h.id == B_UP { i - 1 } else { i + 1 };
            let m = &mut b.m;
            if i < 0 || j < 0 || i >= m.n || j >= m.n {
                return MbResult::None;
            }
            m.part.swap(i as usize, j as usize);
        }
        B_REMOVE => {
            let m = &mut b.m;
            if m.n <= 1 || i < 0 || i >= m.n {
                return MbResult::None;
            }
            let mut k = i;
            while k + 1 < m.n {
                m.part[k as usize] = m.part[k as usize + 1];
                k += 1;
            }
            m.n -= 1;
        }
        B_ADD => {
            let m = &mut b.m;
            if m.n >= MUSIC_MAX_PARTS as i32 {
                return MbResult::None;
            }
            m.part[m.n as usize] = MusicPart(i);
            m.n += 1;
        }
        B_PRESET => {
            let Some(&(_, parts, systems)) = K_PRESETS.get(i as usize) else {
                return MbResult::None;
            };
            let mut m = MusicLayout { systems, ..Default::default() };
            for p in parts.into_iter().filter(|&p| p != PART_NONE) {
                m.part[m.n as usize] = p;
                m.n += 1;
            }
            b.m = m;
            clamp_systems(b, a);
            musicbuild_draw(b, a);
            *dirty = b.rect;
            return MbResult::Changed;
        }
        B_LESS => b.m.systems -= 1,
        B_MORE => b.m.systems += 1,
        B_CANCEL => {
            b.open = false;
            *dirty = b.rect;
            return MbResult::Cancel;
        }
        B_DONE => {
            b.open = false;
            a.settings.music_layout = music_encode(&b.m);
            settings_save(&a.settings);
            *dirty = b.rect;
            return MbResult::Done;
        }
        _ => return MbResult::None,
    }
    if at_max && h.id != B_LESS {
        b.m.systems = fit(b, a);
    }
    clamp_systems(b, a);
    musicbuild_draw(b, a);
    *dirty = b.rect;
    MbResult::Changed
}
