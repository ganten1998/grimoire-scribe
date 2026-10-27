//! ui.h: e-ink widgets. Stateless draw functions + hit tests; screens own the state.
//! Style: white surfaces, 3 px black outlines, themed corners (ui_frame), bold labels, no grays
//! below 128 except ink, so everything stays crisp under GL16/DU refresh.
//! Sizes scale with dpi via ui_px(); at 300 dpi a standard touch target is ~ 10 mm.
//! Icons come from Terminess Nerd Font (its Material Design glyphs); type follows the theme.
//! Text goes through i18n.h's _() and friends (included here for every screen).

use crate::prelude::*;

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
}
impl IconKind {
    /// ICON_COUNT
    pub const COUNT: usize = 53;
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
    todo!()
}
pub fn ui_theme() -> UiTheme {
    todo!()
}

pub fn ui_init(dpi: f32) {
    todo!()
}
/// scales a 300-dpi pixel size to the current dpi
pub fn ui_px(px_at_300dpi: f32) -> i32 {
    todo!()
}
/// body text px (~ 3.4 mm)
pub fn ui_text_size() -> f32 {
    todo!()
}
/// title text px (~ 5 mm)
pub fn ui_title_size() -> f32 {
    todo!()
}
/// small text px (~ 2.6 mm)
pub fn ui_small_size() -> f32 {
    todo!()
}

/// The icon for a drawing tool (ToolKind value).
pub fn ui_tool_icon(tool: i32) -> IconKind {
    todo!()
}
/// Icon glyph scaled to fit r (with a small inset), centered.
pub fn ui_icon(c: &mut Canvas, k: IconKind, r: Rect, gray: u8) {
    todo!()
}
/// The same icon hollow: its outline in edge, a few px wide, around its shape in inside (the
/// surface under it, so the glyph reads as drawn in outline). A pen that draws white ink shows so.
pub fn ui_icon_hollow(c: &mut Canvas, k: IconKind, r: Rect, edge: u8, inside: u8) {
    todo!()
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
/// Icon above label if both given; selected = inverted (black fill, white glyph). The label keeps
/// inside the button's edge, and in Gothic inside the carved inner line of a button at least 90 px
/// each way, 4 px clear of it. A label alone that is too long for its button (a translation) takes
/// a smaller size, down to the small one, then two lines (ui_text).
pub fn ui_button(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle) {
    todo!()
}
/// A button with its icon alone, drawn hollow (ui_icon_hollow) where ui_button draws it: the
/// editor's panel shows the pen in use so while it draws white ink.
pub fn ui_button_hollow(c: &mut Canvas, r: Rect, icon: IconKind, st: ButtonStyle) {
    todo!()
}
/// The size ui_button draws a label alone at in r (the small size where it takes two lines), and
/// such a button with its label at px instead (one line at px, else two): a grid of buttons draws
/// every label at the smallest size any of them needs, rather than each at its own.
pub fn ui_button_text_px(r: Rect, label: &str, st: ButtonStyle) -> f32 {
    todo!()
}
pub fn ui_button_text_at(c: &mut Canvas, r: Rect, label: &str, st: ButtonStyle, px: f32) {
    todo!()
}
/// The width a button h px tall (and at least 90 px wide) needs to show its label whole at its
/// usual size, under its icon or alone: a row of buttons widens to the widest need, so that a
/// longer translation fits.
pub fn ui_button_need_w(h: i32, icon: IconKind, label: &str, st: ButtonStyle) -> i32 {
    todo!()
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
    todo!()
}
pub fn ui_slider_draw(c: &mut Canvas, s: &Slider, value_text: Option<&str>) {
    todo!()
}
/// Maps an x coordinate to a snapped value; returns true if the value changed.
pub fn ui_slider_set_from_x(s: &mut Slider, x: f32) -> bool {
    todo!()
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
    todo!()
}
pub fn ui_slider_drag_to(s: &mut Slider, drag: &mut SliderDrag, x: f32, y: f32, now: u32) -> bool {
    todo!()
}

/// A box in the theme's shape (rounded, or cut corners), same arguments as canvas_round_rect.
pub fn ui_frame(c: &mut Canvas, r: Rect, radius: i32, gray: u8, filled: bool, t: i32) {
    todo!()
}
/// A horizontal rule (gothic: thick over thin).
pub fn ui_rule(c: &mut Canvas, x0: i32, x1: i32, y: i32, t: i32, gray: u8) {
    todo!()
}
/// Gothic only: a cornice of pointed arches h px deep hanging from y.
pub fn ui_frieze(c: &mut Canvas, x0: i32, x1: i32, y: i32, h: i32, gray: u8) {
    todo!()
}
/// Gothic quatrefoil ornament of radius r.
pub fn ui_emblem(c: &mut Canvas, cx: f32, cy: f32, r: f32, ink: u8, bg: u8) {
    todo!()
}

/// Modal sheet: dims nothing (e-ink), draws a white card with a thick border + title bar.
/// Returns the content rect inside the card.
pub fn ui_sheet(c: &mut Canvas, card: Rect, title: &str) -> Rect {
    todo!()
}
/// Toast-style message box centered near the bottom of the screen: one line, or two (taller) for
/// text that needs them; what two lines cannot hold ends in "...".
pub fn ui_toast_rect(screen: Rect, msg: &str) -> Rect {
    todo!()
}
pub fn ui_toast(c: &mut Canvas, screen: Rect, msg: &str) {
    todo!()
}
/// r: ui_toast_rect's, moved as the app needs
pub fn ui_toast_at(c: &mut Canvas, r: Rect, msg: &str) {
    todo!()
}
/// the widest a toast's line can be on that screen (px)
pub fn ui_toast_room(screen: Rect) -> i32 {
    todo!()
}
pub fn ui_checkbox(c: &mut Canvas, r: Rect, label: &str, checked: bool) {
    todo!()
}

/// Text that fits r, for labels a translation may make longer: on one line at px if it can; else
/// at the largest size down to min_px that holds it on one line (font_fit_px: Gothic's pixel face
/// has only whole steps, 44 px to 34 px being the one that counts); else on up to max_lines lines
/// at min_px, as many as r's height holds; only then cut short (which the overflow hook hears of).
/// Centered both ways, or left-aligned and centered vertically. Text that fits is drawn exactly as
/// font_draw_centered or font_draw_left draws it.
pub fn ui_text(c: &mut Canvas, f: FontFace, px: f32, min_px: f32, r: Rect, text: &str, max_lines: i32, center: bool, gray: u8) {
    todo!()
}
/// The width a box needs for its text: the text at px with pad on each side, kept within
/// [min_w, max_w], for a chip or a button that grows with its label instead of cutting it.
pub fn ui_text_box_w(f: FontFace, px: f32, text: &str, pad: i32, min_w: i32, max_w: i32) -> i32 {
    todo!()
}
/// The same for a chip h px tall with ends as round as it can be (ui_frame with radius h / 2):
/// the text keeps clear of Gothic's pointed ends, which are h / 2 deep.
pub fn ui_chip_w(f: FontFace, px: f32, text: &str, h: i32, min_w: i32, max_w: i32) -> i32 {
    todo!()
}
/// A notebook's or folder's name as the screens show it: name itself when the fonts draw all of it,
/// else without what they cannot (font_drawable_copy), and Untitled when no letter is left ("Untitled
/// 2" for a book's title in Japanese and its " 2"). The name stays whole in the files. Returns name
/// or buf (cap bytes).
pub fn ui_name_label<'a>(name: &'a str, buf: &'a mut String, cap: usize) -> &'a str {
    todo!()
}

/// Tap tracking shared by screens: a press that starts and ends in the same rect within
/// slop = tap. Long press = held > 600 ms without moving.
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
    todo!()
}
/// marks moved beyond slop (ui_px(30))
pub fn press_move(p: &mut Press, x: f32, y: f32) {
    todo!()
}
/// call at release
pub fn press_is_tap(p: &Press, r: Rect) -> bool {
    todo!()
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
