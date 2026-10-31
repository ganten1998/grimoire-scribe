//! ui.h: e-ink widgets. Stateless draw functions + hit tests; screens own the state.
//! Style: white surfaces, 3 px black outlines, themed corners (ui_frame), bold labels, no grays
//! below 128 except ink, so everything stays crisp under GL16/DU refresh.
//! Sizes scale with dpi via ui_px(); at 300 dpi a standard touch target is ~ 10 mm.
//! Icons come from Terminess Nerd Font (its Material Design glyphs); type follows the theme.
//! Text goes through i18n.h's _() and friends (included here for every screen).

use crate::prelude::*;
use std::cell::Cell;

// ui.c: widgets, in two themes. Classic: Terminess, rounded corners. Gothic: book and
// blackletter type, cut-stone corners, double rules, lozenges and pointed arches. Icons are
// Terminess Nerd Font in both.

thread_local! {
    static G_SCALE: Cell<f32> = const { Cell::new(1.0) };
    static G_THEME: Cell<UiTheme> = const { Cell::new(UiTheme::Gothic) };
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IconKind {
    #[default]
    None = 0,  // ICON_NONE
    Pencil = 1,  // ICON_PENCIL
    MechPencil = 2,  // ICON_MECH_PENCIL
    Ballpoint = 3,  // ICON_BALLPOINT
    Eraser = 4,  // ICON_ERASER
    Undo = 5,  // ICON_UNDO
    Redo = 6,  // ICON_REDO
    Prev = 7,  // ICON_PREV
    Next = 8,  // ICON_NEXT
    Plus = 9,  // ICON_PLUS
    Minus = 10,  // ICON_MINUS
    Close = 11,  // ICON_CLOSE
    Paper = 12,  // ICON_PAPER
    Gear = 13,  // ICON_GEAR
    Library = 14,  // ICON_LIBRARY
    Collapse = 15,  // ICON_COLLAPSE
    Expand = 16,  // ICON_EXPAND
    Grip = 17,  // ICON_GRIP
    Folder = 18,  // ICON_FOLDER
    Notebook = 19,  // ICON_NOTEBOOK
    More = 20,  // ICON_MORE
    Share = 21,  // ICON_SHARE
    Trash = 22,  // ICON_TRASH
    Check = 23,  // ICON_CHECK
    Back = 24,  // ICON_BACK
    Target = 25,  // ICON_TARGET
    Exit = 26,  // ICON_EXIT
    Tune = 27,  // ICON_TUNE
    Rename = 28,  // ICON_RENAME
    FolderMove = 29,  // ICON_FOLDER_MOVE
    FolderPlus = 30,  // ICON_FOLDER_PLUS
    Download = 31,  // ICON_DOWNLOAD
    Highlighter = 32,  // ICON_HIGHLIGHTER
    Broom = 33,  // ICON_BROOM
    Brightness = 34,  // ICON_BRIGHTNESS
    Invert = 35,  // ICON_INVERT
    Fountain = 36,  // ICON_FOUNTAIN
    Fineliner = 37,  // ICON_FINELINER
    Marker = 38,  // ICON_MARKER
    Up = 39,  // ICON_UP
    Down = 40,  // ICON_DOWN
    Music = 41,  // ICON_MUSIC
    Castle = 42,  // ICON_CASTLE
    Charcoal = 43,  // ICON_CHARCOAL
    Calligraphy = 44,  // ICON_CALLIGRAPHY
    Language = 45,  // ICON_LANGUAGE
    Pin = 46,  // ICON_PIN
    Grid = 47,  // ICON_GRID
    Copy = 48,  // ICON_COPY
    Link = 49,  // ICON_LINK
    Live = 50,  // ICON_LIVE
    Gamepad = 51,  // ICON_GAMEPAD
    Update = 52,  // ICON_UPDATE
    /// the clock and battery corner's (status.rs; not in the C tree)
    Clock = 53,
    /// the Index's (Rust only): Go to page or name, and the link areas' eye
    Search = 54,
    Eye = 55,
    /// the Links bar's New link and Index buttons (Rust only)
    LinkPlus = 56,
    Bubbles = 57,
/// Recorded touch gestures in Settings (Rust only).
    Gestures = 58,
}
impl IconKind {
    /// ICON_COUNT
    pub const COUNT: usize = 59;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UiMetrics {
    pub dpi: f32,
    /// dpi / 300
    pub scale: f32,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UiTheme {
    #[default]
    Gothic = 0,  // THEME_GOTHIC
    Classic = 1,  // THEME_CLASSIC
}
/// also switches the fonts
pub fn ui_set_theme(t: UiTheme) {
    let theme = if t == UiTheme::Classic { UiTheme::Classic } else { UiTheme::Gothic };
    G_THEME.set(theme);
    font_set_gothic(theme == UiTheme::Gothic);
}
pub fn ui_theme() -> UiTheme {
    G_THEME.get()
}

fn gothic() -> bool {
    G_THEME.get() == UiTheme::Gothic
}

/// A box in the theme's shape (rounded, or cut corners), same arguments as canvas_round_rect.
pub fn ui_frame(c: &mut Canvas, r: Rect, radius: i32, gray: u8, filled: bool, t: i32) {
    if gothic() {
        canvas_chamfer_rect(c, r, radius + radius / 4, gray, filled, t);
    } else {
        canvas_round_rect(c, r, radius, gray, filled, t);
    }
}

/// A pointed (equilateral) arch: springers at (x0, ys) and (x1, ys), stroked with width w.
// (3.14159265f is the C's float literal, not f32::consts::PI)
#[allow(clippy::approx_constant, clippy::excessive_precision)]
fn arch(c: &mut Canvas, x0: f32, x1: f32, ys: f32, w: f32, gray: u8) {
    let span = x1 - x0;
    const SEG: i32 = 8;
    for side in 0..2 {
        // Left half: arc centred on the right springer; right half mirrored.
        let ccx = if side == 0 { x1 } else { x0 };
        let a0 = if side == 0 { 3.14159265f32 } else { 0.0 };
        let a1 = if side == 0 { 2.0943951f32 } else { 1.0471976 };
        // (ccx + span * cosf(a0) and ys - span * sinf(a0) are contracted multiply-adds in the C build: fmla)
        let mut px = fmla(span, a0.cos(), ccx);
        let mut py = fmla(-span, a0.sin(), ys);
        for i in 1..=SEG {
            let th = a0 + (a1 - a0) * i as f32 / SEG as f32;
            let qx = fmla(span, th.cos(), ccx);
            let qy = fmla(-span, th.sin(), ys);
            canvas_line_aa(c, px, py, qx, qy, w, gray);
            px = qx;
            py = qy;
        }
    }
}

/// Gothic only: a cornice of pointed arches h px deep hanging from y.
pub fn ui_frieze(c: &mut Canvas, x0: i32, x1: i32, y: i32, h: i32, gray: u8) {
    if !gothic() || x1 - x0 < h {
        return;
    }
    // A cornice of pointed arches hanging from the rule at y, a lozenge drop at every springer.
    let mut span = h as f32 / 0.866;
    let n = ((x1 - x0) as f32 / span) as i32;
    if n < 1 {
        return;
    }
    span = (x1 - x0) as f32 / n as f32;
    let ys = fmla(span, 0.866, y as f32);
    let w = ui_px(3.0) as f32;
    for i in 0..n {
        arch(c, fmla(span, i as f32, x0 as f32), fmla(span, (i + 1) as f32, x0 as f32), ys, w, gray,
        );
    }
    for i in 0..=n {
        let x = fmla(span, i as f32, x0 as f32);
        canvas_line_aa(c, x, y as f32, x, ys, w, gray);
        canvas_lozenge(c, x, ys + ui_px(7.0) as f32, ui_px(7.0) as f32, gray);
    }
}

/// A horizontal rule (gothic: thick over thin).
pub fn ui_rule(c: &mut Canvas, x0: i32, x1: i32, y: i32, t: i32, gray: u8) {
    if !gothic() {
        canvas_hline(c, x0, x1, y, t, gray);
        return;
    }
    // Thick over thin, the way a printer's rule frames a title page.
    canvas_hline(c, x0, x1, y, t, gray);
    canvas_hline(c, x0, x1, y + t + ui_px(5.0), ui_px(2.0), gray);
}

/// An ornamental divider: thin rule broken by a lozenge with two small ones beside it.
fn ornament(c: &mut Canvas, x0: i32, x1: i32, y: i32, gray: u8) {
    let mid = (x0 + x1) / 2;
    let gap = ui_px(60.0);
    canvas_hline(c, x0, mid - gap, y - ui_px(1.0), ui_px(2.0), gray);
    canvas_hline(c, mid + gap, x1, y - ui_px(1.0), ui_px(2.0), gray);
    canvas_lozenge(c, mid as f32, y as f32, ui_px(16.0) as f32, gray);
    canvas_lozenge(c, (mid - ui_px(36.0)) as f32, y as f32, ui_px(8.0) as f32, gray,
    );
    canvas_lozenge(c, (mid + ui_px(36.0)) as f32, y as f32, ui_px(8.0) as f32, gray,
    );
}

/// Gothic quatrefoil ornament of radius r.
pub fn ui_emblem(c: &mut Canvas, cx: f32, cy: f32, r: f32, ink: u8, bg: u8) {
    // Quatrefoil: four overlapping lobes drawn as one outline, a lozenge at the heart.
    let lr = r * 0.5;
    let t = (2.0f32).max(r * 0.09);
    let off: [[f32; 2]; 4] = [[-lr, 0.0], [lr, 0.0], [0.0, -lr], [0.0, lr]];
    for o in &off {
        canvas_disc_aa(c, cx + o[0], cy + o[1], lr, ink, 1.0);
    }
    for o in &off {
        canvas_disc_aa(c, cx + o[0], cy + o[1], lr - t, bg, 1.0);
    }
    canvas_lozenge(c, cx, cy, r * 0.32, ink);
    // (r * 0.32f - t * 1.4f: the first product is contracted with the subtraction, see fmla)
    canvas_lozenge(c, cx, cy, fmla(r, 0.32, -(t * 1.4)), bg);
    canvas_lozenge(c, cx, cy, r * 0.12, ink);
}

pub fn ui_init(dpi: f32) {
    G_SCALE.set(if dpi > 50.0 { dpi / 300.0 } else { 1.0 });
}
/// scales a 300-dpi pixel size to the current dpi
pub fn ui_px(px_at_300dpi: f32) -> i32 {
    (px_at_300dpi * G_SCALE.get()).round_ties_even() as i32
}
/// body text px (~ 3.4 mm)
pub fn ui_text_size() -> f32 {
    44.0 * G_SCALE.get()
}
/// title text px (~ 5 mm)
pub fn ui_title_size() -> f32 {
    64.0 * G_SCALE.get()
}
/// small text px (~ 2.6 mm)
pub fn ui_small_size() -> f32 {
    34.0 * G_SCALE.get()
}

/// Material Design glyphs inside Terminess Nerd Font (codes from nerd-fonts glyphnames.json).
const K_ICON_CP: [u32; IconKind::COUNT] = {
    let mut k = [0u32; IconKind::COUNT];
    k[IconKind::Pencil as usize] = 0xF064F; // md-lead_pencil
    k[IconKind::MechPencil as usize] = 0xF0CB6; // md-pencil_outline
    k[IconKind::Ballpoint as usize] = 0xF03EA; // md-pen
    k[IconKind::Eraser as usize] = 0xF01FE; // md-eraser
    k[IconKind::Undo as usize] = 0xF054C; // md-undo
    k[IconKind::Redo as usize] = 0xF044E; // md-redo
    k[IconKind::Prev as usize] = 0xF0141; // md-chevron_left
    k[IconKind::Next as usize] = 0xF0142; // md-chevron_right
    k[IconKind::Plus as usize] = 0xF0415; // md-plus
    k[IconKind::Minus as usize] = 0xF0374; // md-minus
    k[IconKind::Close as usize] = 0xF0156; // md-close
    k[IconKind::Paper as usize] = 0xF09EE; // md-file_document_outline
    k[IconKind::Gear as usize] = 0xF0493; // md-cog
    k[IconKind::Library as usize] = 0xF125F; // md-bookshelf
    k[IconKind::Collapse as usize] = 0xF0615; // md-arrow_collapse
    k[IconKind::Expand as usize] = 0xF0616; // md-arrow_expand
    k[IconKind::Grip as usize] = 0xF01DB; // md-drag
    k[IconKind::Folder as usize] = 0xF024B; // md-folder
    k[IconKind::Notebook as usize] = 0xF082E; // md-notebook
    k[IconKind::More as usize] = 0xF01D8; // md-dots_horizontal
    k[IconKind::Share as usize] = 0xF05A9; // md-wifi
    k[IconKind::Trash as usize] = 0xF0A7A; // md-trash_can_outline
    k[IconKind::Check as usize] = 0xF012C; // md-check
    k[IconKind::Back as usize] = 0xF004D; // md-arrow_left
    k[IconKind::Target as usize] = 0xF01A3; // md-crosshairs
    k[IconKind::Exit as usize] = 0xF0206; // md-exit_to_app
    k[IconKind::Tune as usize] = 0xF062E; // md-tune
    k[IconKind::Gestures as usize] = 0xF07CB; // md-gesture
    k[IconKind::Rename as usize] = 0xF0455; // md-rename_box
    k[IconKind::FolderMove as usize] = 0xF0252; // md-folder_move
    k[IconKind::FolderPlus as usize] = 0xF0257; // md-folder_plus
    k[IconKind::Download as usize] = 0xF01DA; // md-download
    k[IconKind::Highlighter as usize] = 0xF0652; // md-marker
    k[IconKind::Broom as usize] = 0xF00E2; // md-broom
    k[IconKind::Brightness as usize] = 0xF00DF; // md-brightness_6
    k[IconKind::Invert as usize] = 0xF0301; // md-invert_colors
    k[IconKind::Fountain as usize] = 0xF0D13; // md-fountain_pen_tip
    k[IconKind::Fineliner as usize] = 0xF0F49; // md-draw
    k[IconKind::Marker as usize] = 0xF0648; // md-grease_pencil
    k[IconKind::Up as usize] = 0xF0143; // md-chevron_up
    k[IconKind::Down as usize] = 0xF0140; // md-chevron_down
    k[IconKind::Music as usize] = 0xF075A; // md-music
    k[IconKind::Castle as usize] = 0xF011A; // md-castle
    k[IconKind::Charcoal as usize] = 0xF00B5; // md-blur (grainy dots)
    k[IconKind::Calligraphy as usize] = 0xF06D3; // md-feather (a quill)
    k[IconKind::Language as usize] = 0xF05CA; // md-translate
    k[IconKind::Pin as usize] = 0xF0403; // md-pin
    k[IconKind::Grid as usize] = 0xF0570; // md-view_grid
    k[IconKind::Copy as usize] = 0xF018F; // md-content_copy
    k[IconKind::Link as usize] = 0xF0339; // md-link_variant
    k[IconKind::Live as usize] = 0xF0119; // md-cast_connected
    k[IconKind::Gamepad as usize] = 0xF0297; // md-gamepad_variant
    k[IconKind::Update as usize] = 0xF06B0; // md-update
    k[IconKind::Clock as usize] = 0xF0150; // md-clock_outline
    k[IconKind::Search as usize] = 0xF0349; // md-magnify
    k[IconKind::Eye as usize] = 0xF0208; // md-eye
    k[IconKind::LinkPlus as usize] = 0xF0C94; // md-link_plus
    k[IconKind::Bubbles as usize] = 0xF05E3; // md-chart_bubble
    k
};

/// The icon for a drawing tool (ToolKind value).
pub fn ui_tool_icon(tool: i32) -> IconKind {
    const K: [IconKind; 10] = [
        IconKind::Pencil,
        IconKind::MechPencil,
        IconKind::Ballpoint,
        IconKind::Eraser,
        IconKind::Highlighter,
        IconKind::Fountain,
        IconKind::Fineliner,
        IconKind::Marker,
        IconKind::Charcoal,
        IconKind::Calligraphy,
    ];
    const _: () = assert!(K.len() == TOOL_COUNT, "an icon for every tool");
    if tool >= 0 && (tool as usize) < K.len() {
        K[tool as usize]
    } else {
        IconKind::Pencil
    }
}
/// Icon glyph scaled to fit r (with a small inset), centered.
pub fn ui_icon(c: &mut Canvas, k: IconKind, r: Rect, gray: u8) {
    if k == IconKind::None || K_ICON_CP[k as usize] == 0 {
        return;
    }
    let inset = (if r.w < r.h { r.w } else { r.h }) / 10;
    font_draw_icon(c, FontFace::Regular, K_ICON_CP[k as usize], rect_inflate(r, -inset), gray,
    );
}
/// The same icon hollow: its outline in edge, a few px wide, around its shape in inside (the
/// surface under it, so the glyph reads as drawn in outline). A pen that draws white ink shows so.
pub fn ui_icon_hollow(c: &mut Canvas, k: IconKind, r: Rect, edge: u8, inside: u8) {
    // The glyph drawn in edge all round within t px (a disc of offsets), then in inside where it is.
    let t = ui_px(4.0);
    for dy in -t..=t {
        for dx in -t..=t {
            if (dx != 0 || dy != 0) && dx * dx + dy * dy <= t * t {
                ui_icon(c, k, Rect { x: r.x + dx, y: r.y + dy, w: r.w, h: r.h ,
                    }, edge,
                );
            }
        }
    }
    ui_icon(c, k, r, inside);
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonStyle {
    #[default]
    Plain = 0,  // BTN_PLAIN
    Selected = 1,  // BTN_SELECTED
    Disabled = 2,  // BTN_DISABLED
    Primary = 3,  // BTN_PRIMARY
}
fn radius() -> i32 {
    ui_px(16.0)
}

// In Gothic a button big enough for one has the inner line of a carved panel, 2 px wide, this far
// inside its edge.
fn carved(w: i32, h: i32) -> bool {
    gothic() && h >= ui_px(90.0) && w >= ui_px(90.0)
}
fn carve_in(st: ButtonStyle) -> i32 {
    if st == ButtonStyle::Primary {
        ui_px(13.0)
    } else {
        ui_px(10.0)
    }
}
// How far a label stays from each side of its button: inside the edge, and inside the carved line
// where there is one, clear of it by 4 px (a word up to the edge's inset would run into it).
fn label_inset(carve: bool, st: ButtonStyle) -> i32 {
    if carve {
        carve_in(st) + ui_px(2.0) + ui_px(4.0)
    } else {
        ui_px(8.0)
    }
}
// A label's size under an icon, and a label's alone (before a long one shrinks), for a button
// whose inside (its rect less 8 px all round) is in_h tall.
fn icon_label_px(in_h: i32) -> f32 {
    ui_small_size().min(in_h as f32 * 0.28)
}
fn text_label_px(in_h: i32) -> f32 {
    ui_text_size().min(in_h as f32 * 0.6)
}

// ui_button, with a label alone drawn at text_px when that is not 0 (one line at that size, else
// two), not at the size of its own that fits.
fn button(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle, text_px: f32) {
    let bg: u8 = if st == ButtonStyle::Selected { 0 } else { 255 };
    let fg: u8 = if st == ButtonStyle::Selected {
        255
    } else if st == ButtonStyle::Disabled {
        170
    } else {
        0
    };
    let edge: u8 = if st == ButtonStyle::Disabled { 170 } else { 0 };
    ui_frame(c, r, radius(), bg, true, 0);
    ui_frame(c, r, radius(), edge, false, if st == ButtonStyle::Primary { ui_px(6.0) } else { ui_px(3.0) },
    );
    let carve = carved(r.w, r.h);
    if carve {
        // The inner line of a carved panel.
        let inset = carve_in(st);
        canvas_chamfer_rect(
            c,
            rect_inflate(r, -inset),
            radius() - inset / 3,
            if st == ButtonStyle::Selected { 255 } else { edge },
            false,
            ui_px(2.0),
        );
    }
    let has_label = !label.is_empty();
    let inner = rect_inflate(r, -ui_px(8.0));
    let li = label_inset(carve, st);
    let lx = r.x + li;
    let lw = r.w - 2 * li; // the label's columns
    if icon != IconKind::None && has_label {
        let ts = icon_label_px(inner.h);
        let lh = (ts * 1.15) as i32;
        let ir = Rect { x: inner.x, y: inner.y, w: inner.w, h: inner.h - lh ,
        };
        ui_icon(c, icon, ir, fg);
        let mut lr = Rect { x: lx, y: inner.y + inner.h - lh, w: lw, h: lh ,
        };
        if gothic() {
            lr = Rect { x: lx, y: inner.y + inner.h - lh - ui_px(6.0), w: lw, h: lh ,
            };
        }
        font_draw_centered(c, FontFace::Caps, ts, lr, label, fg);
    } else if icon != IconKind::None {
        ui_icon(c, icon, rect_inflate(inner, -ui_px(6.0)), fg);
    } else if has_label {
        // A label too long for the button (a translation) takes the small size, then two lines.
        let ts = if text_px > 0.0 { text_px } else { text_label_px(inner.h) };
        ui_text(
            c,
            FontFace::Caps,
            ts,
            if text_px > 0.0 { ts } else { ts.min(ui_small_size()) },
            Rect { x: lx, y: inner.y, w: lw, h: inner.h ,
            },
            label,
            2,
            true,
            fg,
        );
    }
}

/// Icon above label if both given; selected = inverted (black fill, white glyph). The label keeps
/// inside the button's edge, and in Gothic inside the carved inner line of a button at least 90 px
/// each way, 4 px clear of it. A label alone that is too long for its button (a translation) takes
/// a smaller size, down to the small one, then two lines (ui_text).
pub fn ui_button(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle) {
    button(c, r, icon, label, st, 0.0);
}
/// A button with its icon alone, drawn hollow (ui_icon_hollow) where ui_button draws it: the
/// editor's panel shows the pen in use so while it draws white ink.
pub fn ui_button_hollow(c: &mut Canvas, r: Rect, icon: IconKind, st: ButtonStyle) {
    button(c, r, IconKind::None, "", st, 0.0);
    let bg: u8 = if st == ButtonStyle::Selected { 0 } else { 255 };
    let fg: u8 = if st == ButtonStyle::Selected {
        255
    } else if st == ButtonStyle::Disabled {
        170
    } else {
        0
    };
    ui_icon_hollow(c, icon, rect_inflate(rect_inflate(r, -ui_px(8.0)), -ui_px(6.0)), fg, bg,
    ); // where button() puts it
}
/// The size ui_button draws a label alone at in r (the small size where it takes two lines), and
/// such a button with its label at px instead (one line at px, else two): a grid of buttons draws
/// every label at the smallest size any of them needs, rather than each at its own.
pub fn ui_button_text_px(r: Rect, label: &str, st: ButtonStyle) -> f32 {
    if label.is_empty() {
        return 0.0;
    }
    let ts = text_label_px(r.h - 2 * ui_px(8.0));
    let min = ts.min(ui_small_size());
    let s = font_fit_px(FontFace::Caps, ts, min, label, r.w - 2 * label_inset(carved(r.w, r.h), st),
    );
    if s > 0.0 {
        s
    } else {
        min
    }
}
pub fn ui_button_text_at(c: &mut Canvas, r: Rect, label: &str, st: ButtonStyle, px: f32) {
    button(c, r, IconKind::None, label, st, px);
}
/// The width a button h px tall (and at least 90 px wide) needs to show its label whole at its
/// usual size, under its icon or alone: a row of buttons widens to the widest need, so that a
/// longer translation fits.
pub fn ui_button_need_w(h: i32, icon: IconKind, label: &str, st: ButtonStyle) -> i32 {
    if label.is_empty() {
        return 0;
    }
    let in_h = h - 2 * ui_px(8.0);
    let ts = if icon != IconKind::None { icon_label_px(in_h) } else { text_label_px(in_h) };
    font_measure(FontFace::Caps, ts, label) + 2 * label_inset(carved(ui_px(90.0), h), st)
}

/// Horizontal slider: label + value text on the left, track on the right.
/// value_text may be NULL (then "%.2f unit" is shown).
#[derive(Clone, Copy, Debug, Default)]
pub struct Slider {
    /// whole row
    pub r: Rect,
    /// filled in by ui_slider_layout
    pub track: Rect,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
    pub label: &'static str,
    pub unit: &'static str,
}
pub fn ui_slider_layout(s: &mut Slider, row: Rect) {
    s.r = row;
    let label_w = row.w * 42 / 100;
    let knob = ui_px(26.0);
    s.track = Rect { x: row.x + label_w + knob, y: row.y + row.h / 2 - ui_px(5.0), w: row.w - label_w - 2 * knob, h: ui_px(10.0) ,
    };
}
fn slider_t(s: &Slider) -> f32 {
    if s.max > s.min {
        clampf((s.value - s.min) / (s.max - s.min), 0.0, 1.0)
    } else {
        0.0
    }
}

fn slider_knob_radius() -> i32 {
    ui_px(if gothic() { 32.0 } else { 26.0 })
}

pub fn ui_slider_draw(c: &mut Canvas, s: &Slider, value_text: Option<&str>) {
    canvas_fill(c, s.r, 255);
    let label_w = s.r.w * 42 / 100;
    let mut vbuf = String::new();
    let value_text: &str = match value_text {
        Some(t) => t,
        None => {
            csnprintf!(&mut vbuf, 48, "%.2f%s%s", s.value, if s.unit.is_empty() { "" } else { " " }, s.unit);
            &vbuf
        }
    };
    let ts = ui_text_size();
    let vw = font_measure(FontFace::Bold, ts, value_text);
    let lr = Rect { x: s.r.x, y: s.r.y, w: label_w - vw - ui_px(16.0), h: s.r.h ,
    };
    font_draw_left(c, FontFace::Regular, ts, lr, s.label, 0);
    font_draw_left(c, FontFace::Bold, ts, Rect { x: s.r.x + label_w - vw, y: s.r.y, w: vw + 2, h: s.r.h ,
        }, value_text, 0,
    );
    let tr = s.track;
    let t = slider_t(s);
    let fx = tr.x + (t * tr.w as f32).round_ties_even() as i32;
    let cy = tr.y as f32 + tr.h as f32 / 2.0;
    if gothic() {
        // A groove of two hairlines, filled solid up to a lozenge knob.
        canvas_hline(c, tr.x, tr.x + tr.w, tr.y, ui_px(2.0), 0);
        canvas_hline(c, tr.x, tr.x + tr.w, tr.y + tr.h - ui_px(2.0), ui_px(2.0), 0,
        );
        canvas_fill(c, Rect { x: tr.x, y: tr.y, w: fx - tr.x, h: tr.h ,
            }, 0,
        );
        canvas_lozenge(c, fx as f32, cy, slider_knob_radius() as f32, 0);
        canvas_lozenge(c, fx as f32, cy, ui_px(20.0) as f32, 255);
        canvas_lozenge(c, fx as f32, cy, ui_px(10.0) as f32, 0);
        return;
    }
    canvas_round_rect(c, tr, tr.h / 2, 190, true, 0);
    canvas_round_rect(c, Rect { x: tr.x, y: tr.y, w: fx - tr.x + tr.h / 2, h: tr.h ,
        }, tr.h / 2, 0, true, 0,
    );
    canvas_disc_aa(c, fx as f32, cy, slider_knob_radius() as f32, 0, 1.0);
    canvas_disc_aa(c, fx as f32, cy, ui_px(12.0) as f32, 255, 1.0);
}
fn slider_set_value(s: &mut Slider, mut v: f32) -> bool {
    if s.step > 0.0 {
        // (min + roundf(..) * step is one contracted multiply-add in the C build: fmla)
        v = fmla(((v - s.min) / s.step).round(), s.step, s.min);
    }
    v = clampf(v, s.min, s.max);
    if (v - s.value).abs() < 1e-6 {
        return false;
    }
    s.value = v;
    true
}

/// Maps an x coordinate to a snapped value; returns true if the value changed.
pub fn ui_slider_set_from_x(s: &mut Slider, x: f32) -> bool {
    let t = if s.track.w > 0 { clampf((x - s.track.x as f32) / s.track.w as f32, 0.0, 1.0) } else { 0.0 };
    slider_set_value(s, fmla(t, s.max - s.min, s.min))
}
/// Within a knob-height band, the value tracks the pointer directly. After a short hold,
/// dragging outside it reduces sensitivity with distance. Fractional motion is retained.
#[derive(Clone, Copy, Debug, Default)]
pub struct SliderDrag {
    pub x: f32,
    pub value: f32,
    pub started: u32,
    pub held: bool,
}
pub fn ui_slider_drag_begin(s: &Slider, drag: &mut SliderDrag, x: f32, now: u32) {
    *drag = SliderDrag { x, value: s.value, started: now, held: false ,
    };
}
pub fn ui_slider_drag_to(s: &mut Slider, drag: &mut SliderDrag, x: f32, y: f32, now: u32) -> bool {
    if now.wrapping_sub(drag.started) >= 350 {
        drag.held = true;
    }
    let distance = (y - fmla(s.track.h as f32, 0.5, s.track.y as f32)).abs();
    let changed;
    // In a band as tall as the visible knob, catch up to the pointer and track it directly.
    // Leaving that band after a hold resumes fine motion from the current value.
    if !drag.held || distance <= slider_knob_radius() as f32 {
        changed = ui_slider_set_from_x(s, x);
        drag.value = s.value;
    } else {
        let gain = (0.01f32).max((0.25f32).powf(distance / ui_px(80.0) as f32));
        if s.track.w > 0 {
            // (value += step * gain: the product is contracted with the addition in the C build: fmla)
            drag.value = fmla((x - drag.x) * (s.max - s.min) / s.track.w as f32, gain, drag.value,
            );
        }
        drag.value = clampf(drag.value, s.min, s.max);
        changed = slider_set_value(s, drag.value);
    }
    drag.x = x;
    changed
}


/// Modal sheet: dims nothing (e-ink), draws a white card with a thick border + title bar.
/// Returns the content rect inside the card.
pub fn ui_sheet(c: &mut Canvas, card: Rect, title: &str) -> Rect {
    if gothic() {
        canvas_chamfer_rect(c, card, ui_px(34.0), 255, true, 0);
        canvas_chamfer_rect(c, card, ui_px(34.0), 0, false, ui_px(6.0));
        let inner = rect_inflate(card, -ui_px(16.0));
        canvas_chamfer_rect(c, inner, ui_px(26.0), 0, false, ui_px(2.0));
        let th = ui_px(130.0);
        if !title.is_empty() {
            font_draw_centered(
                c,
                FontFace::Title,
                ui_title_size(),
                Rect { x: card.x + ui_px(80.0), y: card.y + ui_px(10.0), w: card.w - ui_px(160.0), h: th - ui_px(20.0) ,
                },
                title,
                0,
            );
            ornament(c, card.x + ui_px(60.0), card.x + card.w - ui_px(60.0), card.y + th, 0,
            );
            return rect_inflate(Rect { x: card.x, y: card.y + th, w: card.w, h: card.h - th ,
                }, -ui_px(40.0),
            );
        }
        return rect_inflate(card, -ui_px(44.0));
    }
    canvas_round_rect(c, card, ui_px(24.0), 255, true, 0);
    canvas_round_rect(c, card, ui_px(24.0), 0, false, ui_px(6.0));
    let th = ui_px(120.0);
    if !title.is_empty() {
        font_draw_left(
            c,
            FontFace::Bold,
            ui_title_size(),
            Rect { x: card.x + ui_px(40.0), y: card.y, w: card.w - ui_px(80.0), h: th ,
            },
            title,
            0,
        );
        canvas_hline(c, card.x + ui_px(6.0), card.x + card.w - ui_px(6.0), card.y + th, ui_px(3.0), 0,
        );
        return rect_inflate(Rect { x: card.x, y: card.y + th, w: card.w, h: card.h - th ,
            }, -ui_px(36.0),
        );
    }
    rect_inflate(card, -ui_px(36.0))
}
// A toast is one line of text, or two at the same size when it needs them: the box grows by a
// line, as wide as the longer line needs, and keeps the ends of a one-line box (Gothic's points,
// Classic's round ends), which the text stays clear of.
fn toast_one_h() -> i32 {
    ui_px(110.0)
}
fn toast_inset() -> i32 {
    if gothic() {
        ui_px(50.0)
    } else {
        ui_px(30.0)
    }
}

/// Toast-style message box centered near the bottom of the screen: one line, or two (taller) for
/// text that needs them; what two lines cannot hold ends in "...".
pub fn ui_toast_rect(screen: Rect, msg: &str) -> Rect {
    let ts = ui_text_size();
    let max_w = screen.w - ui_px(60.0);
    let mut w = font_measure(FontFace::Bold, ts, msg) + ui_px(100.0);
    let mut h = toast_one_h();
    if w > max_w {
        w = max_w;
        let mut widest = 0;
        let lines = font_wrap_lines(FontFace::Bold, ts, msg, ui_toast_room(screen), Some(&mut widest),
        );
        if lines > 1 {
            h += font_ascent(FontFace::Bold, ts) + font_descent(FontFace::Bold, ts);
        }
        if lines == 2 && widest + ui_px(100.0) < max_w {
            w = widest + ui_px(100.0);
        }
    }
    Rect { x: screen.x + (screen.w - w) / 2, y: screen.y + screen.h - h - ui_px(120.0), w, h ,
    }
}
pub fn ui_toast(c: &mut Canvas, screen: Rect, msg: &str) {
    ui_toast_at(c, ui_toast_rect(screen, msg), msg);
}
/// r: ui_toast_rect's, moved as the app needs
pub fn ui_toast_at(c: &mut Canvas, r: Rect, msg: &str) {
    let ts = ui_text_size();
    let ends = toast_one_h() / 2; // the ends' depth (or radius): half a one-line box
    let tr = rect_inflate(r, -toast_inset());
    if gothic() {
        // A black banner with pointed ends and a white inner line.
        canvas_chamfer_rect(c, r, ends, 0, true, 0);
        canvas_chamfer_rect(c, rect_inflate(r, -ui_px(9.0)), ends - ui_px(9.0), 255, false, ui_px(2.0),
        );
    } else {
        canvas_round_rect(c, r, ends, 0, true, 0);
    }
    if r.h > toast_one_h() {
        font_draw_wrapped(c, FontFace::Bold, ts, tr, msg, 2, 0, true, 255);
    } else {
        font_draw_centered(c, FontFace::Bold, ts, tr, msg, 255);
    }
}
/// the widest a toast's line can be on that screen (px)
pub fn ui_toast_room(screen: Rect) -> i32 {
    screen.w - ui_px(60.0) - 2 * toast_inset()
}
pub fn ui_checkbox(c: &mut Canvas, r: Rect, label: &str, checked: bool) {
    canvas_fill(c, r, 255);
    let bs = ui_px(64.0);
    let b = Rect { x: r.x, y: r.y + (r.h - bs) / 2, w: bs, h: bs ,
    };
    if gothic() {
        // A lozenge, filled when ticked.
        let cx = b.x as f32 + bs as f32 / 2.0;
        let cy = b.y as f32 + bs as f32 / 2.0;
        let rr = bs as f32 / 2.0 + ui_px(4.0) as f32;
        canvas_lozenge(c, cx, cy, rr, 0);
        canvas_lozenge(c, cx, cy, rr - ui_px(5.0) as f32, 255);
        if checked {
            canvas_lozenge(c, cx, cy, rr - ui_px(14.0) as f32, 0);
        }
    } else {
        canvas_round_rect(c, b, ui_px(10.0), if checked { 0 } else { 255 }, true, 0);
        canvas_round_rect(c, b, ui_px(10.0), 0, false, ui_px(4.0));
        if checked {
            ui_icon(c, IconKind::Check, b, 255);
        }
    }
    ui_text(
        c,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        Rect { x: r.x + bs + ui_px(28.0), y: r.y, w: r.w - bs - ui_px(28.0), h: r.h ,
        },
        label,
        2,
        false,
        0,
    );
}

/// Text that fits r, for labels a translation may make longer: on one line at px if it can; else
/// at the largest size down to min_px that holds it on one line (font_fit_px: Gothic's pixel face
/// has only whole steps, 44 px to 34 px being the one that counts); else on up to max_lines lines
/// at min_px, as many as r's height holds; only then cut short (which the overflow hook hears of).
/// Centered both ways, or left-aligned and centered vertically. Text that fits is drawn exactly as
/// font_draw_centered or font_draw_left draws it.
pub fn ui_text(c: &mut Canvas, f: FontFace, mut px: f32, mut min_px: f32, r: Rect, text: &str, max_lines: i32, center: bool, gray: u8,
) {
    if min_px > px {
        min_px = px;
    }
    if font_measure(f, px, text) > r.w {
        let s = font_fit_px(f, px, min_px, text, r.w);
        if s > 0.0 {
            px = s;
        } else {
            let lh = font_ascent(f, min_px) + font_descent(f, min_px);
            let room = if lh > 0 { r.h / lh } else { 1 };
            font_draw_wrapped(c, f, min_px, r, text, if max_lines < room { max_lines } else if room < 1 { 1 } else { room }, 0, center, gray,
            );
            return;
        }
    }
    if center {
        font_draw_centered(c, f, px, r, text, gray);
    } else {
        font_draw_left(c, f, px, r, text, gray);
    }
}
/// The same adaptive sizing for arbitrary notebook/folder names. Their final ellipsis is
/// intentional, as in font_draw_left_name, rather than a translation-fit error.
pub fn ui_text_name(c:&mut Canvas,f:FontFace,px:f32,min_px:f32,r:Rect,text:&str,max_lines:i32,gray:u8) {
    let min_px=min_px.min(px);
    let size=font_fit_px(f,px,min_px,text,r.w);
    if size>0.0 {font_draw_left_name(c,f,size,r,text,gray);}
    else {
        let lh=(font_ascent(f,min_px)+font_descent(f,min_px)).max(1);
        font_draw_wrapped_name(c,f,min_px,r,text,max_lines.min((r.h/lh).max(1)),gray);
    }
}
/// The width a box needs for its text: the text at px with pad on each side, kept within
/// [min_w, max_w], for a chip or a button that grows with its label instead of cutting it.
pub fn ui_text_box_w(f: FontFace, px: f32, text: &str, pad: i32, min_w: i32, max_w: i32) -> i32 {
    clampi(font_measure(f, px, text) + 2 * pad, min_w, max_w)
}
/// The same for a chip h px tall with ends as round as it can be (ui_frame with radius h / 2):
/// the text keeps clear of Gothic's pointed ends, which are h / 2 deep.
pub fn ui_chip_w(f: FontFace, px: f32, text: &str, h: i32, min_w: i32, max_w: i32) -> i32 {
    // Gothic's ends are points as deep as half the chip (ui_frame's cut corners): the text keeps
    // all but 7 px of that from each tip. Classic's round ends leave the text room closer in.
    ui_text_box_w(f, px, text, if gothic() { h / 2 - ui_px(7.0) } else { ui_px(24.0) }, min_w, max_w,
    )
}
/// A notebook's or folder's name as the screens show it: name itself when the fonts draw all of it,
/// else without what they cannot (font_drawable_copy), and Untitled when no letter is left ("Untitled
/// 2" for a book's title in Japanese and its " 2"). The name stays whole in the files. Returns name
/// or buf (cap bytes).
pub fn ui_name_label<'a>(name: &'a str, buf: &'a mut String, cap: usize) -> &'a str {
    if cap == 0 || font_drawable_copy(name, buf, cap) == 0 {
        return name;
    }
    for &p in buf.as_bytes() {
        if p >= 0x80 || ((p | 0x20) as u32).wrapping_sub('a' as u32) < 26 {
            return buf.as_str(); // a letter is left
        }
    }
    let mut rest = String::new();
    strlcpy_q(&mut rest, buf, 256);
    csnprintf!(buf, cap, "%s%s%s", tr("Untitled"), if rest.is_empty() { "" } else { " " }, &rest);
    buf.as_str()
}

/// How far a finger's tap may roll where it has no button to stay on (a link on the page, the clock's
/// corner): a fingertip pressed firmly on glass rolls a few millimetres.
pub const TAP_SLOP_MM: f32 = 5.0;

/// Tap tracking shared by screens: a press that starts and ends in the same rect is a tap, however
/// the fingertip rolled between (moved, past the slop, is for drags and holds). Long press = held >
/// 600 ms without moving.
#[derive(Clone, Copy, Debug, Default)]
pub struct Press {
    pub down: bool,
    pub x0: f32,
    pub y0: f32,
    pub x: f32,
    pub y: f32,
    pub t0: u32,
    pub moved: bool,
}
pub fn press_begin(p: &mut Press, x: f32, y: f32, t: u32) {
    *p = Press { down: true, x0: x, y0: y, x, y, t0: t, moved: false ,
    };
}
/// marks moved beyond slop (ui_px(30))
pub fn press_move(p: &mut Press, x: f32, y: f32) {
    p.x = x;
    p.y = y;
    let s = ui_px(30.0) as f32;
    if (x - p.x0).abs() > s || (y - p.y0).abs() > s {
        p.moved = true;
    }
}
/// call at release (after press_move to the release's place: a cancelled touch's is off the screen,
/// never a tap)
pub fn press_is_tap(p: &Press, r: Rect) -> bool {
    rect_contains(r, p.x0 as i32, p.y0 as i32) && rect_contains(r, p.x as i32, p.y as i32)
}

/// Hit regions registered while drawing; screens dispatch taps through them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hit {
    pub r: Rect,
    pub id: i32,
    pub arg: i32,
}
/// The C's Hit h[128] and n: at most HIT_MAX hits, the rest ignored.
#[derive(Clone, Debug, Default)]
pub struct HitList {
    pub h: Vec<Hit>,
}
pub const HIT_MAX: usize = 128;
#[inline]
pub fn hits_clear(l: &mut HitList) {
    l.h.clear();
}
#[inline]
pub fn hits_add(l: &mut HitList, r: Rect, id: i32, arg: i32) {
    if l.h.len() < HIT_MAX {
        l.h.push(Hit { r, id, arg });
    }
}
/// Topmost (last added) hit containing the point, or None.
#[inline]
pub fn hits_find(l: &HitList, x: f32, y: f32) -> Option<&Hit> {
    l.h.iter().rev().find(|h| rect_contains(h.r, x as i32, y as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(dpi: f32) {
        ui_init(dpi);
        assert_eq!(font_init(), 0);
    }

    #[test]
    fn sizes_follow_the_dpi() {
        ui_init(150.0);
        assert_eq!(ui_px(100.0), 50);
        assert_eq!(ui_px(3.0), 2); // 1.5 rounds to even
        assert_eq!(ui_px(5.0), 2); // 2.5 too
        assert_eq!(ui_text_size(), 22.0);
        assert_eq!(ui_title_size(), 32.0);
        assert_eq!(ui_small_size(), 17.0);
        // a dpi the panel could not have is taken as 300
        ui_init(30.0);
        assert_eq!(ui_px(100.0), 100);
        ui_init(300.0);
        assert_eq!(ui_text_size(), 44.0);
    }

    #[test]
    fn the_theme_switches_the_fonts() {
        up(300.0);
        // (ui_theme starts out Gothic but font_gothic false, as in the C: the fonts take the theme
        // when ui_set_theme first runs, which the app does at startup)
        ui_set_theme(UiTheme::Gothic);
        assert!(font_gothic());
        ui_set_theme(UiTheme::Classic);
        assert_eq!(ui_theme(), UiTheme::Classic);
        assert!(!font_gothic());
        ui_set_theme(UiTheme::Gothic);
        assert!(font_gothic());
    }

    #[test]
    fn a_tool_has_its_icon() {
        assert_eq!(ui_tool_icon(0), IconKind::Pencil);
        assert_eq!(ui_tool_icon(3), IconKind::Eraser);
        assert_eq!(ui_tool_icon(9), IconKind::Calligraphy);
        assert_eq!(ui_tool_icon(10), IconKind::Pencil);
        assert_eq!(ui_tool_icon(-1), IconKind::Pencil);
        assert_eq!(K_ICON_CP[IconKind::None as usize], 0);
        assert!(K_ICON_CP[1..].iter().all(|&cp| cp != 0));
    }

    #[test]
    fn a_slider_snaps_and_follows_a_drag() {
        up(300.0);
        let mut s = Slider { min: 0.0, max: 10.0, step: 1.0, value: 3.0, label: "Size", unit: "mm", ..Default::default() };
        ui_slider_layout(&mut s, Rect { x: 0, y: 0, w: 1000, h: 90 ,
            },
        );
        assert_eq!(s.r, Rect { x: 0, y: 0, w: 1000, h: 90 });
        assert_eq!(s.track, Rect { x: 420 + 26, y: 45 - 5, w: 1000 - 420 - 52, h: 10 });
        let mid = s.track.x as f32 + s.track.w as f32 * 0.52;
        assert!(ui_slider_set_from_x(&mut s, mid));
        assert_eq!(s.value, 5.0);
        // the same place again changes nothing
        assert!(!ui_slider_set_from_x(&mut s, mid));
        assert!(ui_slider_set_from_x(&mut s, -50.0));
        assert_eq!(s.value, 0.0);
        assert!(ui_slider_set_from_x(&mut s, 5000.0));
        assert_eq!(s.value, 10.0);

        // a drag inside the knob's band tracks the pointer; after a hold, far above it, slowly
        let mut d = SliderDrag::default();
        ui_slider_drag_begin(&s, &mut d, mid, 1000);
        assert_eq!((d.x, d.value, d.started, d.held), (mid, 10.0, 1000, false));
        let cy = s.track.y as f32 + 5.0;
        let (tx, tw) = (s.track.x as f32, s.track.w as f32);
        assert!(ui_slider_drag_to(&mut s, &mut d, tx, cy, 1100));
        assert_eq!(s.value, 0.0);
        assert!(!d.held);
        let x0 = tx + 100.0;
        ui_slider_drag_to(&mut s, &mut d, x0, cy - 400.0, 1400);
        assert!(d.held);
        let before = s.value;
        ui_slider_drag_to(&mut s, &mut d, x0 + 200.0, cy - 400.0, 1500);
        assert!(s.value >= before && s.value < before + 1.0);
        // a hold below the band's edge tracks it again
        ui_slider_drag_to(&mut s, &mut d, tx + tw, cy, 1600);
        assert_eq!(s.value, 10.0);
    }

    #[test]
    fn a_slider_without_text_shows_its_value() {
        up(300.0);
        ui_set_theme(UiTheme::Classic);
        let mut s = Slider { min: 0.0, max: 1.0, step: 0.0, value: 0.25, label: "Opacity", unit: "", ..Default::default() };
        ui_slider_layout(&mut s, Rect { x: 0, y: 0, w: 900, h: 90 ,
            },
        );
        let mut a = Canvas::default();
        canvas_init(&mut a, 900, 90);
        ui_slider_draw(&mut a, &s, None);
        let mut b = Canvas::default();
        canvas_init(&mut b, 900, 90);
        ui_slider_draw(&mut b, &s, Some("0.25"));
        assert_eq!(a.px, b.px);
        s.unit = "mm";
        let mut c = Canvas::default();
        canvas_init(&mut c, 900, 90);
        ui_slider_draw(&mut c, &s, Some("0.25 mm"));
        ui_slider_draw(&mut b, &s, None);
        assert_eq!(b.px, c.px);
    }

    #[test]
    fn a_press_is_a_tap_while_it_stays_in_its_rect() {
        ui_init(300.0);
        let mut p = Press::default();
        press_begin(&mut p, 100.0, 100.0, 7);
        assert!(p.down && !p.moved && p.t0 == 7);
        let r = Rect { x: 50, y: 50, w: 100, h: 100 ,
        };
        press_move(&mut p, 120.0, 90.0);
        assert!(!p.moved);
        assert!(press_is_tap(&p, r));
        // rolled past the slop, still in the rect where it began: moved (no tap for a drag), a tap
        press_move(&mut p, 149.0, 149.0);
        assert!(p.moved);
        assert!(press_is_tap(&p, r));
        // released outside the rect, or cancelled (off the screen), it is not a tap
        press_begin(&mut p, 100.0, 100.0, 8);
        press_move(&mut p, 100.0, 160.0);
        assert!(!press_is_tap(&p, r));
        press_begin(&mut p, 100.0, 100.0, 9);
        press_move(&mut p, -10000.0, -10000.0);
        assert!(!press_is_tap(&p, r));
    }

    #[test]
    fn hits_find_the_topmost_and_stop_at_128() {
        let mut l = HitList::default();
        hits_add(&mut l, Rect { x: 0, y: 0, w: 100, h: 100 ,
            }, 1, 10,
        );
        hits_add(&mut l, Rect { x: 50, y: 50, w: 100, h: 100 ,
            }, 2, 20,
        );
        assert_eq!(hits_find(&l, 60.5, 60.5).map(|h| h.id), Some(2));
        assert_eq!(hits_find(&l, 10.0, 10.0).map(|h| (h.id, h.arg)), Some((1, 10)));
        assert!(hits_find(&l, 200.0, 10.0).is_none());
        assert!(hits_find(&l, -0.5, 10.0).map(|h| h.id) == Some(1)); // (int)-0.5 is 0, as in the C
        for i in 0..200 {
            hits_add(&mut l, Rect { x: 0, y: 0, w: 1, h: 1 ,
                }, i, 0,
            );
        }
        assert_eq!(l.h.len(), HIT_MAX);
        hits_clear(&mut l);
        assert!(l.h.is_empty());
    }

    #[test]
    fn a_toast_grows_a_line_when_it_must() {
        up(300.0);
        ui_set_theme(UiTheme::Classic);
        let screen = Rect { x: 0, y: 0, w: 700, h: 900 ,
        };
        let short = ui_toast_rect(screen, "Saved");
        assert_eq!(short.h, ui_px(110.0));
        assert_eq!(short.y + short.h, 900 - ui_px(120.0));
        assert_eq!(short.x, (700 - short.w) / 2);
        let two = ui_toast_rect(screen, "A rather long message that will need two lines to be shown on this screen",
        );
        assert!(two.h > short.h);
        assert!(two.w <= 700 - ui_px(60.0));
        assert_eq!(ui_toast_room(screen), 700 - ui_px(60.0) - 2 * ui_px(30.0));
        let mut c = Canvas::default();
        canvas_init(&mut c, 700, 900);
        ui_toast(&mut c, screen, "Saved");
        assert_eq!(c.px[((short.y + short.h / 2) * c.stride + short.x + 5) as usize], 0);
        assert_eq!(c.px[0], 255);
    }

    #[test]
    fn a_sheet_returns_the_room_inside_its_card() {
        up(300.0);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            let card = Rect { x: 50, y: 60, w: 600, h: 500 ,
            };
            let mut c = Canvas::default();
            canvas_init(&mut c, 700, 650);
            let with = ui_sheet(&mut c, card, "Settings");
            let without = ui_sheet(&mut c, card, "");
            assert!(rect_contains_rect(card, with) && rect_contains_rect(card, without));
            assert!(with.h < without.h && with.y > without.y);
        }
    }

    #[test]
    fn button_labels_shrink_then_wrap() {
        up(300.0);
        ui_set_theme(UiTheme::Classic);
        let r = Rect { x: 0, y: 0, w: 200, h: 140 ,
        };
        let own = ui_button_text_px(r, "Save", ButtonStyle::Plain);
        assert_eq!(own, ui_text_size().min((140 - 2 * ui_px(8.0)) as f32 * 0.6));
        let long = ui_button_text_px(r, "A label that is far too long to fit this button", ButtonStyle::Plain,
        );
        assert_eq!(long, ui_small_size().min(own));
        assert_eq!(ui_button_text_px(r, "", ButtonStyle::Plain), 0.0);
        assert_eq!(ui_button_need_w(140, IconKind::None, "", ButtonStyle::Plain), 0);
        let need = ui_button_need_w(140, IconKind::None, "Save", ButtonStyle::Plain);
        assert_eq!(need, font_measure(FontFace::Caps, own, "Save") + 2 * ui_px(8.0));
        // the Classic frame does not carve; the Gothic one does, and keeps the label further in
        ui_set_theme(UiTheme::Gothic);
        assert!(ui_button_need_w(140, IconKind::None, "Save", ButtonStyle::Plain) > 0);
        assert!(carved(ui_px(90.0), ui_px(90.0)));
        assert!(!carved(ui_px(89.0), ui_px(200.0)));
    }

    #[test]
    fn text_boxes_and_names() {
        up(300.0);
        ui_set_theme(UiTheme::Classic);
        let w = font_measure(FontFace::Bold, 34.0, "Hello");
        assert_eq!(ui_text_box_w(FontFace::Bold, 34.0, "Hello", 10, 0, 1000), w + 20);
        assert_eq!(ui_text_box_w(FontFace::Bold, 34.0, "Hello", 10, 500, 1000), 500);
        assert_eq!(ui_text_box_w(FontFace::Bold, 34.0, "Hello", 10, 0, 50), 50);
        assert_eq!(ui_chip_w(FontFace::Bold, 34.0, "Hello", 80, 0, 1000), w + 2 * ui_px(24.0));
        ui_set_theme(UiTheme::Gothic);
        let w = font_measure(FontFace::Bold, 34.0, "Hello");
        assert_eq!(ui_chip_w(FontFace::Bold, 34.0, "Hello", 80, 0, 1000), w + 2 * (40 - ui_px(7.0)));
        // a name the fonts draw whole is the name itself
        let mut buf = String::new();
        assert_eq!(ui_name_label("Notebook 3", &mut buf, 64), "Notebook 3");
        assert_eq!(ui_name_label("Notebook", &mut buf, 0), "Notebook");
    }
}
