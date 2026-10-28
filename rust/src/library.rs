//! library.c: the home screen. Folders and notebooks as cards, paged (no scrolling on e-ink),
//! plus the sheets: new notebook (paper picker), settings, notebook actions, move, delete.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Modal {
    #[default]
    None = 0, // MOD_NONE
    New = 1,         // MOD_NEW
    Settings = 2,    // MOD_SETTINGS
    More = 3,        // MOD_MORE
    FolderMore = 4,  // MOD_FOLDER_MORE
    Move = 5,        // MOD_MOVE
    Delete = 6,      // MOD_DELETE
    Kb = 7,          // MOD_KB
    Display = 8,     // MOD_DISPLAY
    Music = 9,       // MOD_MUSIC
    Language = 10,   // MOD_LANGUAGE
    Templates = 11,  // MOD_TEMPLATES
    Koreader = 12,   // MOD_KOREADER
    Shortcuts = 13,  // MOD_SHORTCUTS
    Buttons = 14,    // MOD_BUTTONS
    Pair = 15,       // MOD_PAIR
    Updates = 16,    // MOD_UPDATES
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum KbPurpose {
    #[default]
    NewName = 0, // KBP_NEW_NAME
    Rename = 1,        // KBP_RENAME
    NewFolder = 2,     // KBP_NEW_FOLDER
    MoveNewFolder = 3, // KBP_MOVE_NEW_FOLDER
    RenameFolder = 4,  // KBP_RENAME_FOLDER
}

const H_NONE: i32 = 0;
const H_BACK: i32 = 1;
const H_NEW: i32 = 2;
const H_NEWFOLDER: i32 = 3;
const H_SHARE: i32 = 4;
const H_SETTINGS: i32 = 5;
const H_EXIT: i32 = 6;
const H_PREV: i32 = 7;
const H_NEXT: i32 = 8;
const H_CARD: i32 = 9;
const H_CARD_MORE: i32 = 10;
const H_PAPER: i32 = 11;
const H_PAPER_CUSTOM: i32 = 12;
const H_NEW_RENAME: i32 = 13;
const H_NEW_CREATE: i32 = 14;
const H_CANCEL: i32 = 15;
const H_SET_PENS: i32 = 16;
const H_SET_CALIB: i32 = 17;
const H_SET_MIRROR: i32 = 18;
const H_SET_SIDE: i32 = 19;
const H_SET_ROTATE: i32 = 20;
const H_SET_DISPLAY: i32 = 21;
const H_SET_PINCH: i32 = 22;
const H_SET_THEME: i32 = 23;
const H_SET_LANGUAGE: i32 = 24;
const H_LANG_PICK: i32 = 25;
const H_LANG_BACK: i32 = 26;
const H_MORE_OPEN: i32 = 27;
const H_MORE_RENAME: i32 = 28;
const H_MORE_MOVE: i32 = 29;
const H_MORE_PDF: i32 = 30;
const H_MORE_DELETE: i32 = 31;
const H_FMORE_RENAME: i32 = 32;
const H_FMORE_REMOVE: i32 = 33;
const H_MOVE_TO: i32 = 34;
const H_MOVE_TOP: i32 = 35;
const H_MOVE_NEW: i32 = 36;
const H_DELETE_YES: i32 = 37;
const H_PAPER_MORE: i32 = 38;
const H_KOREADER: i32 = 39;
const H_KO_ITEM: i32 = 40;
const H_SHORTCUTS: i32 = 41;
const H_SHORTCUT_ENABLE: i32 = 42;
const H_SHORTCUT_RELOAD: i32 = 43;
const H_BUTTONS: i32 = 44;
const H_BUTTON_GROUP: i32 = 45;
const H_BUTTON_ROW: i32 = 46;
const H_BUTTONS_RESET: i32 = 47;
const H_BUTTONS_DONE: i32 = 48;
const H_BT_TOGGLE: i32 = 49;
const H_PAIR: i32 = 50;
const H_PAIR_FOUND: i32 = 51;
const H_PAIR_FORGET: i32 = 52;
const H_PAIR_AGAIN: i32 = 53;
const H_UPDATES: i32 = 54;
const H_UPD_CHANNEL: i32 = 55;
const H_UPD_CHECK: i32 = 56;
const H_UPD_NOW: i32 = 57;

#[derive(Clone, Copy, Default)]
struct Item {
    is_folder: bool,
    /// index into books (notebooks)
    nb: usize,
    folder: FixedStr<STORE_NAME_LEN>,
    /// notebooks inside (folders)
    count: i32,
}

/// The screen's state (library.c's, which app.h keeps opaque): library_init makes it, in a.lib.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct LibraryState {
    books: Vec<NotebookInfo>,
    folders: Vec<String>,
    items: Vec<Item>,
    /// "" = top level
    cur_folder: FixedStr<STORE_NAME_LEN>,
    page: i32,
    /// per notebook, lazily loaded (px empty until then)
    thumbs: Vec<Canvas>,
    thumb_tried: Vec<bool>,
    /// thumbs[i] is the notebook's picture paper (picture_card), not its thumb.pgm
    thumb_picture: Vec<bool>,
    paper_prev: [Canvas; PAPER_COUNT as usize],
    /// one custom music layout's preview at a time
    custom_prev: Canvas,
    custom_prev_k: PaperKind,
    mb: MusicBuild,
    /// New notebook's More papers...: the Templates sheet over it
    ts: TplSheet,
    /// the template New notebook makes its notebook on ("" none)
    new_tpl: FixedStr<TEMPLATE_PATH_MAX>,
    /// and its preview, for its tile
    tpl_prev: Canvas,
    /// bgimage_generation when the cards were last read
    card_gen: u32,
    /// per notebook: what thumbs[i] was read from (store_thumb_stamp), or CARD_STALE
    card_stamp: Vec<u64>,
    /// times a background job changed a card on screen (the probe's)
    cards_redrawn: i32,

    shown_shortcut_code: i32,
    shown_shortcut_mods: i32,
    shown_shortcut_gamepad: bool,
    /// App.hid_changes as the Controller sheet's status line (or Buttons' rows) last showed
    shown_hid_changes: u32,
    // The Buttons sheet: its group open (K_GROUPS; -1 the list of them), the row learning its
    // buttons (-1 none), which of its actions, and since when (it gives up after LEARN_MS without a
    // press).
    buttons_group: i32,
    learn_row: i32,
    learn_step: i32,
    learn_at: u32,
    // The Pair sheet (Settings > Controller > Pair a controller): what it asked the Bluetooth helper
    // last (ASK_*), what it asks next and when, the controllers found and those paired.
    bt_asked: i32,
    bt_next: i32,
    bt_next_at: u32,
    scan_until: u32,
    pair_until: u32,
    scanning: bool,
    pairing: bool,
    pair_addr: FixedStr<BT_ADDR_LEN>,
    found: [BtDevice; 4],
    paired: [BtDevice; 2],
    nfound: i32,
    npaired: i32,
    shown_bt_changes: u32,
    // The Updates sheet: its check (UPD_*), the newest version it found, why it failed.
    upd: i32,
    /// at most UPDATE_VERSION_LEN - 1 bytes
    upd_latest: String,
    /// at most 15 bytes
    upd_why: String,
    modal: Modal,
    kbp: KbPurpose,
    kb: Keyboard,
    dm: DispMenu,
    /// item index for MORE / MOVE / DELETE
    target: i32,
    new_paper: PaperKind,
    new_name: FixedStr<STORE_NAME_LEN>,
    #[allow(dead_code)] // (the C keeps it and reads it nowhere)
    move_page: i32,

    hits: HitList,
    press: Press,
    /// The hit the finger went down on (the C's pressed, which points at its pressed_copy).
    pressed: Option<Hit>,
    /// when the finger went down on it (a Buttons row held ROW_HOLD_MS: forget_row)
    pressed_at: u32,
}

/// The C's S (a->lib).
fn lib(a: &mut App) -> &mut LibraryState {
    a.lib.as_deref_mut().unwrap()
}

/// The size of a name as the screens show it (ui_name_label's buffer).
const LABEL_CAP: usize = STORE_NAME_LEN + 32;

// ---- layout ----
fn top_h() -> i32 {
    ui_px(190.0)
}
fn bottom_h() -> i32 {
    ui_px(170.0)
}
fn margin() -> i32 {
    ui_px(50.0)
}
fn grid_top() -> i32 {
    // The Gothic frieze hangs below the header rule; leave air under its drops.
    top_h() + margin() + if ui_theme() == UiTheme::Gothic { ui_px(24.0) } else { 0 }
}

/// cols, rows, cw, ch for a screen aw x ah.
fn grid_geom(aw: i32, ah: i32) -> (i32, i32, i32, i32) {
    let cols = if aw > ah { 5 } else { 3 }; // more, smaller cards when the Kindle is sideways
    let cw = (aw - margin() * (cols + 1)) / cols;
    let th = cw * 4 / 3;
    let ch = th + ui_px(150.0);
    let avail = ah - grid_top() - bottom_h();
    let mut rows = avail / (ch + margin());
    if rows < 1 {
        rows = 1;
    }
    (cols, rows, cw, ch)
}

fn per_page(a: &App) -> i32 {
    let (c, r, _, _) = grid_geom(a.w, a.h);
    c * r
}

// Whether an empty Library (or folder) says so, and what to do: not under a sheet (a short one,
// the language list, would cut through the lines), but above the keyboard, which leaves them be.
fn empty_hint_shown(s: &LibraryState) -> bool {
    s.items.is_empty() && (s.modal == Modal::None || s.modal == Modal::Kb)
}

// ---- data ----

fn free_lists(s: &mut LibraryState) {
    for t in s.thumbs.iter_mut() {
        canvas_free(t);
    }
    s.thumbs = Vec::new();
    s.thumb_tried = Vec::new();
    s.thumb_picture = Vec::new();
    s.card_stamp = Vec::new();
    s.books = Vec::new();
    s.folders = Vec::new();
    s.items = Vec::new();
}

fn reload(a: &mut App) {
    let pp = per_page(a);
    let s = lib(a);
    free_lists(s);
    s.card_gen = bgimage_generation(); // (the cards read from now on are up to date)
    store_list(&mut s.books);
    store_list_folders(&mut s.folders);
    let nbooks = s.books.len();
    s.thumbs = vec![Canvas::default(); nbooks];
    s.thumb_tried = vec![false; nbooks];
    s.thumb_picture = vec![false; nbooks];
    s.card_stamp = vec![0; nbooks];
    s.items = Vec::with_capacity(nbooks + s.folders.len() + 1);
    if s.cur_folder.is_empty() {
        for f in 0..s.folders.len() {
            let mut it = Item { is_folder: true, ..Default::default() };
            it.folder.set(&s.folders[f]);
            for i in 0..nbooks {
                it.count += (s.books[i].folder == s.folders[f].as_str()) as i32;
            }
            s.items.push(it);
        }
    }
    for i in 0..nbooks {
        if s.books[i].folder == s.cur_folder {
            s.items.push(Item { nb: i, ..Default::default() });
        }
    }
    let pages = (s.items.len() as i32 + pp - 1) / pp;
    s.page = clampi(s.page, 0, if pages > 0 { pages - 1 } else { 0 });
}

fn unique_name(s: &LibraryState, base: &FixedStr<STORE_NAME_LEN>, out: &mut FixedStr<STORE_NAME_LEN>) {
    out.set_bytes(base.as_bytes());
    for k in 2..1000 {
        let mut taken = false;
        let mut i = 0;
        while i < s.books.len() && !taken {
            taken = s.books[i].name == *out;
            i += 1;
        }
        if !taken {
            return;
        }
        // "%s %d" of base and k, cut as snprintf cuts it (bytes: the name reaches notebook.ini).
        let mut t = base.as_bytes().to_vec();
        t.extend_from_slice(cformat!(" %d", k).as_bytes());
        out.set_bytes(&t);
    }
}

fn paper_preview(s: &mut LibraryState, dpi: f32, mut k: PaperKind) -> Option<&Canvas> {
    // A newer version's paper (also as a card's fallback), and an image paper: that is one of a
    // notebook's own pictures (picture_card), here blank.
    if !paper_is_known(k) || paper_is_image(k) {
        k = PAPER_BLANK;
    }
    let c: &mut Canvas = if paper_is_custom(k) {
        if !s.custom_prev.px.is_empty() && s.custom_prev_k != k {
            canvas_free(&mut s.custom_prev);
        }
        s.custom_prev_k = k;
        &mut s.custom_prev
    } else {
        &mut s.paper_prev[clampi(k.0 as i32, 0, PAPER_COUNT - 1) as usize]
    };
    if c.px.is_empty() {
        let mut full = Canvas::default();
        let (mut pw, mut ph) = (0, 0);
        store_page_size(&mut pw, &mut ph);
        if canvas_init(&mut full, pw, ph) != 0 {
            return None;
        }
        paper_render(&mut full, k, Rect::new(0, 0, pw, ph), dpi);
        if canvas_init(c, pw / 4, ph / 4) == 0 {
            canvas_downsample_keep_lines(c, &full, 0.6);
        }
        canvas_free(&mut full);
    }
    if c.px.is_empty() {
        None
    } else {
        Some(&*c)
    }
}

// A card drawn with a picture white in it (its file not made yet): read again once a background
// job has ended (library_tick). No thumb.pgm has this stamp (store_thumb_stamp: 0 or odd).
const CARD_STALE: u64 = 2;

// The card of a notebook with no thumbnail yet whose paper is a picture: that picture (white while
// its file is missing), never another paper.
fn picture_card(s: &mut LibraryState, nb: usize) {
    let (mut pw, mut ph) = (0, 0);
    store_notebook_page_size(&s.books[nb], &mut pw, &mut ph);
    let mut full = Canvas::default();
    if canvas_init(&mut full, pw, ph) != 0 {
        return;
    }
    bgimage_take_missing(); // (what drew white before)
    bgimage_render(&mut full, s.books[nb].id.as_str(), s.books[nb].paper, Rect::new(0, 0, pw, ph));
    if bgimage_take_missing() && nb < s.card_stamp.len() {
        s.card_stamp[nb] = CARD_STALE;
    }
    if canvas_init(&mut s.thumbs[nb], pw / 4, ph / 4) == 0 {
        canvas_downsample_keep_lines(&mut s.thumbs[nb], &full, 0.6);
    }
    canvas_free(&mut full);
    s.thumb_picture[nb] = !s.thumbs[nb].px.is_empty();
}

fn thumb_for(s: &mut LibraryState, dpi: f32, nb: usize) -> Option<&Canvas> {
    if !s.thumb_tried[nb] {
        s.thumb_tried[nb] = true;
        s.thumb_picture[nb] = false;
        if nb < s.card_stamp.len() {
            s.card_stamp[nb] = store_thumb_stamp(s.books[nb].id.as_str()); // (before it is read)
        }
        if store_load_thumb(s.books[nb].id.as_str(), &mut s.thumbs[nb]) != 0 && paper_is_image(s.books[nb].paper) {
            picture_card(s, nb);
        }
    }
    if !s.thumbs[nb].px.is_empty() {
        return Some(&s.thumbs[nb]);
    }
    let paper = s.books[nb].paper;
    paper_preview(s, dpi, paper)
}

// ---- drawing ----

fn draw_card(c: &mut Canvas, s: &mut LibraryState, dpi: f32, r: Rect, idx: usize) {
    let it = s.items[idx];
    let th = r.w * 4 / 3;
    let tb = Rect::new(r.x, r.y, r.w, th);
    ui_frame(c, tb, ui_px(16.0), 255, true, 0);
    if it.is_folder {
        ui_icon(c, IconKind::Folder, rect_inflate(tb, -r.w / 5), 0);
    } else {
        let t = thumb_for(s, dpi, it.nb);
        let saved = c.clip;
        canvas_set_clip(c, rect_inflate(tb, -ui_px(3.0)));
        if let Some(t) = t {
            canvas_blit_fit(c, t, rect_inflate(tb, -ui_px(3.0)));
        }
        c.clip = saved;
    }
    ui_frame(c, tb, ui_px(16.0), 0, false, ui_px(4.0));
    let mut line2 = String::new();
    let mut label = String::new();
    let title: &str;
    if it.is_folder {
        title = ui_name_label(it.folder.as_str(), &mut label, LABEL_CAP);
        // TRANSLATORS: under a folder's card in the Library.
        csnprintf!(&mut line2, 128, ng_("%d notebook", "%d notebooks", it.count as i64), it.count);
    } else {
        let nb = &s.books[it.nb];
        title = ui_name_label(nb.name.as_str(), &mut label, LABEL_CAP);
        let mut when = String::new();
        format_time_short(nb.modified, &mut when, 64);
        // TRANSLATORS: under a notebook's card in the Library: its pages and when it last changed (a
        // date and time, "Oct 10, 14:32"). Sideways the line has 316 px at 34 px; when it is longer,
        // the time is left out.
        csnprintf!(&mut line2, 128, ng_("%d page, %s", "%d pages, %s", nb.npages() as i64), nb.npages(), &when);
    }
    let more = ui_px(110.0);
    let tr_ = Rect::new(r.x, r.y + th + ui_px(14.0), r.w - more - ui_px(10.0), ui_px(70.0));
    // Gothic names use the pixel face's regular weight: its overprinted bold turns a name in
    // capitals into a solid black bar.
    let face = if ui_theme() == UiTheme::Gothic { FontFace::Regular } else { FontFace::Bold };
    font_draw_left_name(c, face, ui_text_size(), tr_, title, 0);
    let lr = Rect::new(r.x, tr_.y + tr_.h, r.w - more - ui_px(10.0), ui_px(56.0));
    let mut ls = ui_small_size();
    if font_measure(FontFace::Regular, ls, &line2) > lr.w {
        // Too long for the card (sideways in Classic, or in a longer language): a size smaller by
        // up to a fifth where the face has one (Classic's), else the date without its time.
        let fs = font_fit_px(FontFace::Regular, ls, ls * 0.8, &line2, lr.w);
        if fs > 0.0 {
            ls = fs;
        } else if !it.is_folder {
            let nb = &s.books[it.nb];
            let mut day = String::new();
            format_date_short(nb.modified, &mut day, 64);
            csnprintf!(&mut line2, 128, ng_("%d page, %s", "%d pages, %s", nb.npages() as i64), nb.npages(), &day);
        }
    }
    font_draw_left(c, FontFace::Regular, ls, lr, &line2, 60);
    let mr = Rect::new(r.x + r.w - more, r.y + th + ui_px(20.0), more, ui_px(110.0));
    ui_button(c, mr, IconKind::More, "", ButtonStyle::Plain);
    hits_add(&mut s.hits, tb, H_CARD, idx as i32);
    hits_add(&mut s.hits, mr, H_CARD_MORE, idx as i32);
}

fn draw_base(a: &mut App) {
    let (aw, ah, dpi) = (a.w, a.h, a.dpi);
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    hits_clear(&mut s.hits);
    canvas_fill(c, Rect::new(0, 0, aw, ah), 255);
    let th = top_h();
    let m = margin();
    let mut bw = ui_px(170.0);
    let bh = ui_px(150.0);
    let by = (th - bh) / 2;
    let mut x = m;
    if !s.cur_folder.is_empty() {
        let br = Rect::new(x, by, bh, bh);
        ui_button(c, br, IconKind::Back, "", ButtonStyle::Plain);
        hits_add(&mut s.hits, br, H_BACK, 0);
        x += bh + ui_px(30.0);
    }
    struct Btn {
        icon: IconKind,
        label: &'static str,
        id: i32,
        show: bool,
    }
    let mut btns = [
        // TRANSLATORS: the Library's top bar, a word under each icon, at 34 px: 132 px in Gothic
        // (138 for all but New) and 154 px in Classic. A longer word widens all five buttons, up to
        // 182 px in Gothic (188 for all but New) and 204 px in Classic. Exit leaves Grimoire.
        Btn { icon: IconKind::Exit, label: tr("Exit"), id: H_EXIT, show: true },
        // TRANSLATORS: the Library's top bar (see Exit).
        Btn { icon: IconKind::Gear, label: tr("Settings"), id: H_SETTINGS, show: true },
        // TRANSLATORS: the Library's top bar (see Exit): to the Share screen (over Wi-Fi).
        Btn { icon: IconKind::Share, label: tr("Share"), id: H_SHARE, show: true },
        // TRANSLATORS: the Library's top bar (see Exit): makes a new folder.
        Btn { icon: IconKind::FolderPlus, label: tr("Folder"), id: H_NEWFOLDER, show: s.cur_folder.is_empty() },
        // TRANSLATORS: the Library's top bar (see Exit): makes a new notebook.
        Btn { icon: IconKind::Plus, label: tr("New"), id: H_NEW, show: true },
    ];
    // Started from KOReader's Grimoire plugin (kindle/app/grimoire.sh --hosted koreader), Exit goes
    // back to KOReader.
    let hosted = std::env::var_os("GRIMOIRE_HOST").is_some_and(|h| h == "koreader");
    if hosted {
        // TRANSLATORS: the Library's top bar when Grimoire was opened from KOReader: in place of Exit,
        // it goes back to KOReader. This one button takes the width it needs.
        btns[0].label = tr("Back to KOReader");
    }
    const NBTNS: usize = 5;
    // A word too long for its button (a translation) widens all five alike, as far as leaves the
    // app's name room in the bar (beyond that a word is cut, and the fit check hears of it).
    // Hosted, Exit's longer "Back to KOReader" takes only its own room.
    for (i, b) in btns.iter().enumerate().skip(if hosted { 1 } else { 0 }) {
        let need = ui_button_need_w(bh, b.icon, b.label, if i == 4 { ButtonStyle::Primary } else { ButtonStyle::Plain });
        if b.show {
            bw = clampi(need, bw, ui_px(220.0));
        }
    }
    let mut rx = aw - m;
    for (i, b) in btns.iter().enumerate().take(NBTNS) {
        if !b.show {
            continue;
        }
        let mut w = bw;
        if i == 0 && hosted {
            // as wide as its label needs, never narrower than the rest
            let need = ui_button_need_w(bh, btns[0].icon, btns[0].label, ButtonStyle::Plain);
            if need > w {
                w = need;
            }
        }
        rx -= w;
        let r = Rect::new(rx, by, w, bh);
        ui_button(c, r, b.icon, b.label, if i == 4 { ButtonStyle::Primary } else { ButtonStyle::Plain });
        hits_add(&mut s.hits, r, b.id, 0);
        rx -= ui_px(20.0);
    }
    if !s.cur_folder.is_empty() {
        ui_icon(c, IconKind::Folder, Rect::new(x, by + ui_px(25.0), ui_px(100.0), ui_px(100.0)), 0);
        x += ui_px(115.0);
    } else if ui_theme() == UiTheme::Gothic {
        ui_emblem(c, (x + ui_px(55.0)) as f32, th as f32 / 2.0, ui_px(52.0) as f32, 0, 255);
        x += ui_px(140.0);
    }
    // (A folder's name, cut short if it must be; else the app's.)
    let mut label = String::new();
    let title_px = ui_title_size() * if ui_theme() == UiTheme::Gothic { 1.8 } else { 1.25 };
    let title = if !s.cur_folder.is_empty() { ui_name_label(s.cur_folder.as_str(), &mut label, LABEL_CAP) } else { APP_NAME };
    font_draw_left_name(c, FontFace::Title, title_px, Rect::new(x, 0, rx - x - ui_px(20.0), th), title, 0);
    ui_rule(c, 0, aw, th, ui_px(4.0), 0);
    ui_frieze(c, 0, aw, th + ui_px(9.0), ui_px(28.0), 0);

    let (cols, rows, cw, ch) = grid_geom(aw, ah);
    let pp = cols * rows;
    if empty_hint_shown(s) {
        let mut er = Rect::new(m, th + ui_px(300.0), aw - 2 * m, ui_px(100.0));
        // TRANSLATORS: the Library with nothing in it (1760 px at 64 px), and what to do (at 44 px).
        let empty = if !s.cur_folder.is_empty() { tr("This folder is empty") } else { tr("No notebooks yet") };
        font_draw_centered(c, FontFace::Bold, ui_title_size(), er, empty, 0);
        er.y += ui_px(110.0);
        // TRANSLATORS: New is the top bar's button (as translated there).
        font_draw_centered(c, FontFace::Regular, ui_text_size(), er, tr("Tap New to start one."), 60);
    }
    for i in 0..pp {
        let idx = s.page * pp + i;
        if idx >= s.items.len() as i32 {
            break;
        }
        let col = i % cols;
        let row = i / cols;
        draw_card(c, s, dpi, Rect::new(m + col * (cw + m), grid_top() + row * (ch + m), cw, ch), idx as usize);
    }
    // Pager.
    let mut pages = (s.items.len() as i32 + pp - 1) / pp;
    if pages < 1 {
        pages = 1;
    }
    let y = ah - bottom_h();
    ui_rule(c, 0, aw, y - ui_px(10.0), ui_px(3.0), 0);
    let pr = Rect::new(m, y + ui_px(15.0), ui_px(220.0), bottom_h() - ui_px(30.0));
    let nr = Rect::new(aw - m - ui_px(220.0), pr.y, ui_px(220.0), pr.h);
    ui_button(c, pr, IconKind::Prev, "", if s.page > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    ui_button(c, nr, IconKind::Next, "", if s.page < pages - 1 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if s.page > 0 {
        hits_add(&mut s.hits, pr, H_PREV, 0);
    }
    if s.page < pages - 1 {
        hits_add(&mut s.hits, nr, H_NEXT, 0);
    }
    let mut pl = String::new();
    // TRANSLATORS: between the Library's page buttons (1320 px at 44 px).
    csnprintf!(&mut pl, 96, tr("Page %d of %d"), s.page + 1, pages);
    font_draw_centered(c, FontFace::Bold, ui_text_size(), Rect::new(pr.x + pr.w, pr.y, nr.x - pr.x - pr.w, pr.h), &pl, 0);
}

fn sheet_rect(aw: i32, ah: i32, mut h: i32) -> Rect {
    let m = ui_px(70.0);
    if h > ah - 2 * ui_px(40.0) {
        h = ah - 2 * ui_px(40.0); // landscape: keep it on screen
    }
    let mut w = aw - 2 * m;
    if w > ui_px(1800.0) {
        w = ui_px(1800.0);
    }
    Rect::new((aw - w) / 2, (ah - h) / 3, w, h)
}

fn draw_new(a: &mut App) {
    let (aw, ah, dpi) = (a.w, a.h, a.dpi);
    let App { screen: c, lib, settings, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let card = sheet_rect(aw, ah, ui_px(2300.0));
    // TRANSLATORS: the title of the sheet that makes a notebook, and its heading over the papers.
    let in_ = ui_sheet(c, card, tr("New notebook"));
    font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(in_.x, in_.y, in_.w, ui_px(70.0)), tr("Paper"), 0);
    // The tiles over the name rows and the buttons (420 px), under the heading (80): upright in 6
    // columns, sideways in 7 (tplsheet.h).
    let gap = ui_px(30.0);
    let g = paper_grid(
        PAPER_SHEET_TILES,
        in_.w,
        in_.h - ui_px(80.0) - ui_px(420.0),
        gap,
        ui_px(100.0),
        ui_px(300.0),
        PAPER_GRID_MIN_COLS,
    );
    let gridw = g.cols * g.tw + (g.cols - 1) * gap;
    let ty = in_.y + ui_px(80.0);
    let tpl = !s.new_tpl.is_empty();
    let new_custom = !tpl && paper_is_custom(s.new_paper);
    for i in 0..PAPER_SHEET_TILES {
        // The last two tiles. Custom music: the chosen layout, or the last one built. More papers:
        // the template chosen on the Templates sheet it opens (S->new_paper is then what the
        // notebook is made on if the template cannot be used).
        let custom = i == PAPER_COUNT;
        let more = i == PAPER_COUNT + 1;
        let k = if more {
            PAPER_BLANK
        } else if custom {
            if new_custom {
                s.new_paper
            } else {
                settings.music_layout
            }
        } else {
            paper_at(i)
        };
        let tr_ = Rect::new(in_.x + (in_.w - gridw) / 2 + (i % g.cols) * (g.tw + gap), ty + (i / g.cols) * g.cell_h, g.tw, g.th);
        canvas_fill(c, tr_, 255);
        let pv = if more {
            if tpl && !s.tpl_prev.px.is_empty() {
                Some(&s.tpl_prev)
            } else {
                None
            }
        } else {
            paper_preview(s, dpi, k)
        };
        if let Some(pv) = pv {
            canvas_blit_fit(c, pv, tr_);
        } else if more {
            paper_tile_more(c, tr_);
        }
        let sel = if more {
            tpl
        } else if custom {
            new_custom
        } else {
            !tpl && s.new_paper == k
        };
        ui_frame(c, rect_inflate(tr_, if sel { ui_px(10.0) } else { 0 }), ui_px(12.0), 0, false, if sel { ui_px(9.0) } else { ui_px(3.0) });
        let mut label = paper_label(k);
        if custom {
            // TRANSLATORS: the last paper's tile but one, here and in the editor's paper sheet: it opens
            // a sheet that builds a music paper from parts (192 px at 31 px in New notebook sideways, on
            // one line or two).
            label = tr("Custom music...");
        }
        if more {
            // TRANSLATORS: the last paper's tile, here and in the editor's paper sheet: it opens the
            // Templates sheet, of pictures copied into grimoire/templates to write on as paper (192 px
            // at 31 px in New notebook sideways, on one line or two).
            label = tr("More papers...");
        }
        // (A label wider than its tile goes on two lines under it, as most do sideways: labels side
        // by side never run into each other.)
        paper_tile_label(c, tr_, gap, label, sel);
        let id = if more {
            H_PAPER_MORE
        } else if custom {
            H_PAPER_CUSTOM
        } else {
            H_PAPER
        };
        hits_add(&mut s.hits, Rect::new(tr_.x, tr_.y, tr_.w, g.th + ui_px(90.0)), id, k.0 as i32);
    }
    let mut y = ty + g.rows * g.cell_h + ui_px(20.0);
    let mut line = String::new();
    // TRANSLATORS: the new notebook's name, then its Rename button (268 px at 29 px, under its
    // icon, inside Gothic's carved line), then the folder it goes in, or the Library itself.
    csnprintf!(&mut line, 320, tr("Name: %s"), &s.new_name);
    let rn = Rect::new(in_.x + in_.w - ui_px(300.0), y, ui_px(300.0), ui_px(120.0));
    font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(in_.x, y, in_.w - ui_px(330.0), ui_px(120.0)), &line, 0);
    ui_button(c, rn, IconKind::Rename, tr("Rename"), ButtonStyle::Plain);
    hits_add(&mut s.hits, rn, H_NEW_RENAME, 0);
    y += ui_px(140.0);
    csnprintf!(&mut line, 320, tr("In: %s"), if !s.cur_folder.is_empty() { s.cur_folder.as_str() } else { tr("Library (top level)") });
    font_draw_left(c, FontFace::Regular, ui_text_size(), Rect::new(in_.x, y, in_.w, ui_px(80.0)), &line, 60);
    let bh = ui_px(140.0);
    let bw = ui_px(380.0);
    let cancel = Rect::new(in_.x, in_.y + in_.h - bh, bw, bh);
    let create = Rect::new(in_.x + in_.w - bw, in_.y + in_.h - bh, bw, bh);
    // TRANSLATORS: the sheets' buttons: 348 px at 44 px inside Gothic's carved line (342 for Create,
    // the main one), or at 34 px, or two lines of that.
    ui_button(c, cancel, IconKind::None, tr("Cancel"), ButtonStyle::Plain);
    ui_button(c, create, IconKind::None, tr("Create"), ButtonStyle::Primary);
    hits_add(&mut s.hits, cancel, H_CANCEL, 0);
    hits_add(&mut s.hits, create, H_NEW_CREATE, 0);
}

/// A row of draw_menu's list (the C's MenuRow, its label owned).
struct MenuRow {
    icon: IconKind,
    label: String,
    id: i32,
    arg: i32,
    st: ButtonStyle,
}

fn menu_row(icon: IconKind, label: &str, id: i32, arg: i32, st: ButtonStyle) -> MenuRow {
    MenuRow { icon, label: label.to_string(), id, arg, st }
}

// ---- the controller's buttons (Settings > Controller > Buttons) ----

const LEARN_MS: u32 = 10000;
// A Buttons row held this long without moving loses its buttons (forget_row), as a link held in the
// editor opens (LINK_HOLD_MS).
const ROW_HOLD_MS: u32 = 600;

// A row learns one action's button, or two in turn (the previous page's, then the next page's).
// label None: the action's own name (action_label).
#[derive(Clone, Copy)]
struct LearnRow {
    label: Option<&'static str>,
    act: [i32; 2],
    icon: IconKind,
}

const fn lr(label: Option<&'static str>, act: [i32; 2], icon: IconKind) -> LearnRow {
    LearnRow { label, act, icon }
}

// The Buttons sheet's groups, as the editor's panel groups its buttons: each a sheet of rows.
static K_EDITING: [LearnRow; 7] = [
    lr(None, [SHORTCUT_UNDO, 0], IconKind::Undo),
    lr(None, [SHORTCUT_REDO, 0], IconKind::Redo),
    lr(None, [SHORTCUT_TOOL_BASE + TOOL_ERASER.0, 0], IconKind::Eraser),
    lr(None, [SHORTCUT_HOLD_ERASER, 0], IconKind::Gamepad),
    lr(None, [SHORTCUT_ERASER_MODE, 0], IconKind::None),
    lr(None, [SHORTCUT_WHITE, 0], IconKind::None),
    lr(None, [SHORTCUT_CLEAR_PAGE, 0], IconKind::Broom),
];
// The Tools group: TOOL_ORDER's pens first (learn_row_at), then this.
static K_TOOLS: [LearnRow; 2] = [
    // TRANSLATORS: (Buttons sheet) the previous and the next tool's buttons, in that order (as the
    // other rows, about 1100 px at 44 px in bold, with the buttons' names). They go through the pens
    // on the panel (the tool button's and the pinned ones): a pin beside it.
    lr(Some(n_("Previous / next tool")), [SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL], IconKind::Pin),
    // TRANSLATORS: (Buttons sheet) the buttons that move the open tool picker's choice to its
    // previous and its next pen (with it closed, they switch tools through all nine), in that order.
    lr(Some(n_("Previous / next in the picker")), [SHORTCUT_PICKER_PREV, SHORTCUT_PICKER_NEXT], IconKind::Grid),
];
static K_PAGES: [LearnRow; 8] = [
    // TRANSLATORS: (Buttons sheet) the previous and the next page's buttons, in that order.
    lr(Some(n_("Previous / next page")), [SHORTCUT_PREV_PAGE, SHORTCUT_NEXT_PAGE], IconKind::Next),
    // TRANSLATORS: (Buttons sheet) the first and the last page's buttons, in that order.
    lr(Some(n_("First / last page")), [SHORTCUT_FIRST_PAGE, SHORTCUT_LAST_PAGE], IconKind::None),
    lr(None, [SHORTCUT_NEW_PAGE, 0], IconKind::Plus),
    lr(None, [SHORTCUT_FIT_PAGE, 0], IconKind::Expand),
    lr(None, [SHORTCUT_PAGES, 0], IconKind::Notebook),
    lr(None, [SHORTCUT_INDEX, 0], IconKind::None),
    lr(None, [SHORTCUT_ADD_LINK, 0], IconKind::Link),
    lr(None, [SHORTCUT_NAME_PAGE, 0], IconKind::Rename),
];
static K_MENUS: [LearnRow; 8] = [
    lr(None, [SHORTCUT_TOOL_PICKER, 0], IconKind::Grid),
    lr(None, [SHORTCUT_PAPER, 0], IconKind::Paper),
    lr(None, [SHORTCUT_DISPLAY, 0], IconKind::Brightness),
    lr(None, [SHORTCUT_PEN_FEEL, 0], IconKind::Tune),
    lr(None, [SHORTCUT_FOLD_PANEL, 0], IconKind::Collapse),
    lr(None, [SHORTCUT_LIBRARY, 0], IconKind::Library),
    lr(None, [SHORTCUT_CLOSE_MENU, 0], IconKind::Close),
    lr(None, [SHORTCUT_STATUS, 0], IconKind::Clock),
];
struct Group {
    /// its row in the list
    label: &'static str,
    /// the title of its sheet
    title: &'static str,
    icon: IconKind,
    /// library.buttons_group
    probe: &'static str,
    rows: &'static [LearnRow],
}
const N_GROUPS: i32 = 4;
static K_GROUPS: [Group; N_GROUPS as usize] = [
    // TRANSLATORS: the Buttons sheet's list (Settings > Controller > Buttons): its groups of actions,
    // each opening a sheet of them (about 1100 px at 44 px in bold), and those sheets' titles. Editing:
    // undo, redo, the eraser, white ink, clearing the page.
    Group { label: n_("Editing..."), title: n_("Editing"), icon: IconKind::Undo, probe: "editing", rows: &K_EDITING },
    // TRANSLATORS: (Buttons sheet) its group of the drawing tools, one by one.
    Group { label: n_("Tools..."), title: n_("Tools"), icon: IconKind::Grid, probe: "tools", rows: &K_TOOLS },
    // TRANSLATORS: (Buttons sheet) its group of page turning, new pages, the Pages view and the
    // Index, and links and page names.
    Group { label: n_("Pages..."), title: n_("Pages"), icon: IconKind::Next, probe: "pages", rows: &K_PAGES },
    // TRANSLATORS: (Buttons sheet) its group of the editor's menus: each button opens one (and closes
    // it again).
    Group { label: n_("Menus..."), title: n_("Menus"), icon: IconKind::More, probe: "menus", rows: &K_MENUS },
];

// Whether group g's rows are the Tools group's (which the C tells by its rows' address).
fn is_tools(g: i32) -> bool {
    std::ptr::eq(K_GROUPS[g as usize].rows.as_ptr(), K_TOOLS.as_ptr())
}

// How many rows group g has (the Tools group's pens too).
fn group_rows(g: i32) -> i32 {
    K_GROUPS[g as usize].rows.len() as i32 + if is_tools(g) { TOOL_PENS as i32 } else { 0 }
}

// Row i of group g: the Tools group's first rows are TOOL_ORDER's pens, the picker's order.
fn learn_row_at(g: i32, i: i32) -> LearnRow {
    if is_tools(g) && i < TOOL_PENS as i32 {
        let t = TOOL_ORDER[i as usize];
        return LearnRow { label: None, act: [SHORTCUT_TOOL_BASE + t.0, 0], icon: ui_tool_icon(t.0) };
    }
    K_GROUPS[g as usize].rows[(if is_tools(g) { i - TOOL_PENS as i32 } else { i }) as usize]
}

// An action's name as the Buttons sheet shows it: its row's, and in what it asks while it learns.
fn action_label(action: i32) -> &'static str {
    if action >= SHORTCUT_TOOL_BASE && action < SHORTCUT_TOOL_BASE + TOOL_COUNT as i32 {
        return tool_label(ToolKind(action - SHORTCUT_TOOL_BASE));
    }
    struct Name {
        action: i32,
        label: &'static str,
        /// its msgctxt, for a name the Index shares in English
        ctx: Option<&'static str>,
    }
    const fn nm(action: i32, label: &'static str) -> Name {
        Name { action, label, ctx: None }
    }
    static NAMES: &[Name] = &[
        // TRANSLATORS: the Buttons sheet's rows (Settings > Controller > Buttons), each followed by
        // the controller buttons that do it ("Undo: South"): about 1100 px at 44 px in bold, with the
        // names. The same names fill "Press a button for %s" and "That button is no longer for %s.".
        nm(SHORTCUT_UNDO, n_("Undo")),
        nm(SHORTCUT_REDO, n_("Redo")),
        // TRANSLATORS: (Buttons sheet) the pen erases while the controller's button is held down.
        nm(SHORTCUT_HOLD_ERASER, n_("Erase while held")),
        // TRANSLATORS: (Buttons sheet) the eraser takes whole strokes, or only what it touches, in turn.
        nm(SHORTCUT_ERASER_MODE, n_("Eraser mode")),
        nm(SHORTCUT_WHITE, n_("White ink")),
        // TRANSLATORS: (Buttons sheet) takes every stroke off the page, as the panel's broom.
        nm(SHORTCUT_CLEAR_PAGE, n_("Clear page")),
        nm(SHORTCUT_PREV_TOOL, n_("Previous tool")),
        nm(SHORTCUT_NEXT_TOOL, n_("Next tool")),
        // TRANSLATORS: (Buttons sheet) the tool picker's previous and next pen ("Press a button for
        // Next in the picker").
        nm(SHORTCUT_PICKER_PREV, n_("Previous in the picker")),
        nm(SHORTCUT_PICKER_NEXT, n_("Next in the picker")),
        nm(SHORTCUT_PREV_PAGE, n_("Previous page")),
        nm(SHORTCUT_NEXT_PAGE, n_("Next page")),
        nm(SHORTCUT_FIRST_PAGE, n_("First page")),
        nm(SHORTCUT_LAST_PAGE, n_("Last page")),
        nm(SHORTCUT_NEW_PAGE, n_("New page")),
        // TRANSLATORS: (Buttons sheet) the page's whole width again after a pinch zoomed it.
        nm(SHORTCUT_FIT_PAGE, n_("Fit page")),
        nm(SHORTCUT_PAGES, n_("Pages")),
        // TRANSLATORS: (Buttons sheet) the Pages view's Index tab, the list of named pages.
        nm(SHORTCUT_INDEX, n_("Index")),
        // TRANSLATORS: (Buttons sheet) starts a link on the open page, as the Index's Add a link
        // does: what the pen draws next is its box, then a tap picks where it goes ("Press a button
        // for Add a link").
        Name { action: SHORTCUT_ADD_LINK, label: nc_("buttons", "Add a link"), ctx: Some("buttons") },
        // TRANSLATORS: (Buttons sheet) the naming keyboard for the open page, as the Index's Name
        // this page opens it.
        nm(SHORTCUT_NAME_PAGE, n_("Name the page")),
        // TRANSLATORS: (Buttons sheet) the editor's tool picker, which its tool button opens.
        nm(SHORTCUT_TOOL_PICKER, n_("Tool picker")),
        nm(SHORTCUT_PAPER, n_("Paper")),
        nm(SHORTCUT_DISPLAY, n_("Display")),
        // TRANSLATORS: (Buttons sheet) the screen of each tool's pressure and tilt ("Pen feel: %s").
        nm(SHORTCUT_PEN_FEEL, n_("Pen feel")),
        // TRANSLATORS: (Buttons sheet) folds the editor's panel into its small box, or unfolds it.
        nm(SHORTCUT_FOLD_PANEL, n_("Fold the panel")),
        nm(SHORTCUT_LIBRARY, n_("Library")),
        // TRANSLATORS: (Buttons sheet) closes whichever of the editor's menus is open.
        nm(SHORTCUT_CLOSE_MENU, n_("Close menu")),
        // TRANSLATORS: (Buttons sheet) turns the small clock and battery in the screen's top left
        // corner on or off, on any screen.
        nm(SHORTCUT_STATUS, n_("Clock and battery")),
    ];
    for n in NAMES {
        if n.action == action {
            return match n.ctx {
                Some(ctx) => c_(ctx, n.label),
                None => tr(n.label),
            };
        }
    }
    "?"
}

// The action the open row waits for a press for.
fn learning_action(s: &LibraryState) -> i32 {
    learn_row_at(s.buttons_group, s.learn_row).act[s.learn_step as usize]
}

// "South, Up" for an action's buttons, "none" when it has none; "B, Up" while the first controller
// connected is a Switch-mode one, as its buttons are printed.
fn pads_text(a: &App, action: i32, out: &mut String, cap: usize) {
    let mut codes = [0i32; 4];
    let n = shortcuts_pads_for(&a.shortcuts, action, &mut codes);
    let mut h = [InputHid::default(); 4];
    let nh = input_hids(&mut h);
    let mut switch_layout = false;
    for hid in h.iter().take(nh.max(0) as usize) {
        if hid.gamepad {
            switch_layout = hid.switch_layout;
            break;
        }
    }
    out.clear();
    let mut i = 0;
    while i < n && i < 4 {
        let mut name = String::new();
        shortcut_pad_label(codes[i as usize], switch_layout, &mut name, 16);
        let len = out.len();
        let mut piece = String::new();
        csnprintf!(&mut piece, cap - len, "%s%s", if i != 0 { ", " } else { "" }, &name);
        out.push_str(&piece);
        i += 1;
    }
    if n == 0 {
        // TRANSLATORS: (Buttons sheet) an action no controller button does.
        csnprintf!(out, cap, "%s", c_("buttons", "none"));
    }
}

fn controller_path() -> String {
    let mut out = String::new();
    csnprintf!(&mut out, 1024, "%s/controller.ini", &store_root());
    out
}

// The shortcuts again: shortcuts.ini's keys (and its buttons while there is no controller.ini), and
// gestures.ini's gestures.
fn reload_shortcuts(a: &mut App) {
    let mut sp = String::new();
    csnprintf!(&mut sp, 1024, "%s/shortcuts.ini", &store_root());
    let cp = controller_path();
    shortcuts_load_all(&mut a.shortcuts, &sp, &cp);
    app_gestures_load(a);
}

/// A controller's press while Settings > Controller > Buttons learns one: true when it took it.
// A controller's press while the Buttons sheet learns: true when it was taken (the row's action has
// that button now, or the file could not be written: a toast says so).
pub fn library_learn_key(a: &mut App, code: i32, gamepad: bool) -> bool {
    let Some(s) = a.lib.as_deref() else { return false };
    if s.modal != Modal::Buttons || s.buttons_group < 0 || s.learn_row < 0 {
        return false;
    }
    if !gamepad {
        return true; // (a keyboard's key is shortcuts.ini's: nothing to learn here)
    }
    let cp = controller_path();
    let action = learning_action(s);
    let before = shortcuts_lookup(&a.shortcuts, true, code, 0);
    if shortcuts_learn(&mut a.shortcuts, &cp, code, action) != 0 {
        // TRANSLATORS: a toast: the Buttons sheet could not write its file (the storage is full).
        app_toast(a, tr("Could not save controller.ini"), 4000);
        lib(a).learn_row = -1;
        redraw(a, RefreshMode::Ui);
        return true;
    }
    if before != 0 && before != action {
        let mut msg = String::new();
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the button just learned
        // did something else, which it no longer does; %s is that action's name, as the Buttons sheet
        // names it ("Undo", "Pencil").
        csnprintf!(&mut msg, 200, tr("That button is no longer for %s."), action_label(before));
        app_toast(a, &msg, 2500);
    }
    let s = lib(a);
    if s.learn_step == 0 && learn_row_at(s.buttons_group, s.learn_row).act[1] != 0 {
        s.learn_step = 1;
        s.learn_at = now_ms();
    } else {
        s.learn_row = -1;
    }
    redraw(a, RefreshMode::Ui);
    true
}

// A Buttons row held (ROW_HOLD_MS): its actions lose their buttons, this row's alone, and it reads
// "none"; a toast says so. Its release starts no learning.
fn forget_row(a: &mut App, row: i32) {
    let r = learn_row_at(lib(a).buttons_group, row);
    let mut msg = String::new();
    let cp = controller_path();
    lib(a).learn_row = -1;
    if shortcuts_forget(&mut a.shortcuts, &cp, r.act[0], r.act[1]) != 0 {
        app_toast(a, tr("Could not save controller.ini"), 4000);
    } else {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a finger held on a
        // Buttons sheet row, whose actions no controller button does now; %s is the row's name as the
        // sheet shows it ("Undo", "Previous / next page").
        csnprintf!(&mut msg, 200, tr("No buttons for %s now."), match r.label { Some(l) => tr(l), None => action_label(r.act[0]) });
        app_toast(a, &msg, 2000);
    }
    redraw(a, RefreshMode::Ui);
}

// The Controller sheet's status line: the controllers and keyboards Grimoire reads now.
fn hid_status(a: &mut App, out: &mut String, cap: usize) {
    let mut h = [InputHid::default(); 4];
    let n = input_hids(&mut h);
    let hid_changes = a.hid_changes;
    lib(a).shown_hid_changes = hid_changes;
    if n == 0 {
        // TRANSLATORS: the Controller sheet's status line: no controller or keyboard Grimoire reads now
        // (about 1100 px at 44 px in bold, and the same for the two below).
        csnprintf!(out, cap, "%s", if a.settings.shortcuts_enabled { tr("No controller or keyboard connected") } else { "" });
    } else if h[0].held {
        // TRANSLATORS: (Controller sheet) %s is the controller's own name, as it reports it: another app
        // (a button mapper) holds it, so Grimoire gets none of its presses.
        csnprintf!(out, cap, tr("%s: held by another app"), &h[0].name);
    } else {
        // TRANSLATORS: (Controller sheet) %s is the controller's or keyboard's own name.
        csnprintf!(out, cap, tr("%s connected"), &h[0].name);
    }
    if n > 1 {
        let len = out.len();
        let mut more = String::new();
        csnprintf!(&mut more, cap - len, " (+%d)", n - 1);
        out.push_str(&more);
    }
}

// ---- pairing a controller (the Bluetooth helper's API, bluetooth.h) ----

const ASK_NONE: i32 = 0;
const ASK_STATUS: i32 = 1;
const ASK_SCAN: i32 = 2;
const ASK_SCAN_STATUS: i32 = 3;
const ASK_PAIR: i32 = 4;
const ASK_PAIR_STATUS: i32 = 5;
const ASK_REMOVE: i32 = 6;
const SCAN_MS: u32 = 15000;
const PAIR_MS: u32 = 30000;

// Asks the helper what (an ASK_*) after `in_` ms, or as soon as the one under way is answered.
fn bt_ask(s: &mut LibraryState, what: i32, in_: u32) {
    s.bt_next = what;
    s.bt_next_at = now_ms().wrapping_add(in_);
}

fn bt_send(s: &mut LibraryState) {
    if s.bt_next == 0 || bt_busy() || (now_ms().wrapping_sub(s.bt_next_at) as i32) < 0 {
        return;
    }
    let mut path = String::new();
    let what = s.bt_next;
    match what {
        ASK_STATUS => {
            csnprintf!(&mut path, 160, "/status");
        }
        ASK_SCAN => {
            csnprintf!(&mut path, 160, "/scan");
        }
        ASK_SCAN_STATUS => {
            csnprintf!(&mut path, 160, "/scan-status");
        }
        ASK_PAIR_STATUS => {
            csnprintf!(&mut path, 160, "/pair-status");
        }
        ASK_PAIR => {
            let mut classic = false;
            for d in s.found.iter().take(s.nfound.max(0) as usize) {
                if d.address == s.pair_addr {
                    classic = d.classic;
                }
            }
            csnprintf!(&mut path, 160, "/pair?addr=%s&protocol=%s", &s.pair_addr, if classic { "classic" } else { "ble" });
        }
        ASK_REMOVE => {
            csnprintf!(&mut path, 160, "/remove?addr=%s", &s.pair_addr);
        }
        _ => return,
    }
    if !bt_request(&path) {
        return;
    }
    s.bt_asked = what;
    s.bt_next = ASK_NONE;
}

fn pair_open(s: &mut LibraryState) {
    s.nfound = 0;
    s.npaired = 0;
    s.scanning = true;
    s.pairing = false;
    s.scan_until = now_ms().wrapping_add(SCAN_MS);
    s.bt_asked = ASK_NONE;
    bt_ask(s, ASK_SCAN, 0);
    s.modal = Modal::Pair;
}

/// The Bluetooth helper's news (bluetooth.h) while the library is the screen: its Pair sheet's.
// The helper's answer (or none) to what the Pair sheet asked.
pub fn library_bt(a: &mut App, event: BtEvent, reply: &BtReply) {
    let (e, r) = (event, reply);
    let Some(s) = a.lib.as_deref_mut() else { return };
    if e == BtEvent::State {
        if s.modal == Modal::Shortcuts || s.modal == Modal::Pair {
            redraw(a, RefreshMode::Ui);
        }
        return;
    }
    let asked = s.bt_asked;
    s.bt_asked = ASK_NONE;
    if s.modal != Modal::Pair {
        return;
    }
    let ok = e == BtEvent::Reply && r.ok;
    match asked {
        ASK_SCAN => {
            if ok {
                bt_ask(s, ASK_SCAN_STATUS, 1000);
            } else {
                s.scanning = false;
            }
        }
        ASK_SCAN_STATUS => {
            if e == BtEvent::Reply {
                s.nfound = 0;
                let mut i = 0;
                while i < r.ndev && s.nfound < 4 {
                    s.found[s.nfound as usize] = r.dev[i as usize];
                    s.nfound += 1;
                    i += 1;
                }
            }
            if ok && r.scanning && (now_ms().wrapping_sub(s.scan_until) as i32) < 0 {
                bt_ask(s, ASK_SCAN_STATUS, 1000);
            } else {
                s.scanning = false;
                bt_ask(s, ASK_STATUS, 0); // and which are paired
            }
        }
        ASK_STATUS => {
            s.npaired = 0;
            let mut i = 0;
            while e == BtEvent::Reply && i < r.ndev && s.npaired < 2 {
                s.paired[s.npaired as usize] = r.dev[i as usize];
                s.npaired += 1;
                i += 1;
            }
        }
        ASK_PAIR => {
            if ok {
                bt_ask(s, ASK_PAIR_STATUS, 1000);
            } else {
                s.pairing = false;
                app_toast(a, tr("Could not pair that controller"), 3000);
            }
        }
        ASK_PAIR_STATUS => {
            if ok && r.pairing && (now_ms().wrapping_sub(s.pair_until) as i32) < 0 {
                bt_ask(s, ASK_PAIR_STATUS, 1000);
            } else {
                s.pairing = false;
                // TRANSLATORS: toasts as pairing a controller ends (Settings > Controller > Pair a controller).
                app_toast(a, if ok && !r.pairing { tr("Paired. The controller connects in a moment.") } else { tr("Could not pair that controller") }, 3000);
                bt_ask(lib(a), ASK_STATUS, 0);
            }
        }
        ASK_REMOVE => {
            // TRANSLATORS: a toast: the controller is no longer paired with the Kindle.
            app_toast(a, if ok { tr("Forgotten.") } else { tr("Could not forget that controller") }, 2500);
            bt_ask(lib(a), ASK_STATUS, 0);
        }
        _ => {}
    }
    redraw(a, RefreshMode::Ui);
}

// ---- Settings > Updates (update.h) ----

const UPD_IDLE: i32 = 0;
const UPD_CHECKING: i32 = 1;
const UPD_DONE: i32 = 2;
const UPD_FAILED: i32 = 3;

// Which channel's updates: the setting, or by the version (a beta or an RC: preview).
fn upd_preview(a: &App) -> bool {
    a.settings.updates == 2 || (a.settings.updates == 0 && update_is_prerelease(GRIMOIRE_VERSION))
}

fn upd_newer(s: &LibraryState) -> bool {
    s.upd == UPD_DONE && update_version_cmp(&s.upd_latest, GRIMOIRE_VERSION) > 0
}

// The update check's news (from library_tick).
fn upd_poll(a: &mut App) {
    let s = lib(a);
    if s.upd != UPD_CHECKING {
        return;
    }
    let e = update_tick(&mut s.upd_latest, &mut s.upd_why, 16);
    if e == UpdateEvent::None {
        return;
    }
    s.upd = if e == UpdateEvent::Done { UPD_DONE } else { UPD_FAILED };
    if s.modal == Modal::Updates {
        redraw(a, RefreshMode::Ui);
    }
}

// A vertical list of full-width buttons inside a sheet.
fn draw_menu(a: &mut App, title: &str, rows: &[MenuRow]) {
    let n = rows.len() as i32;
    let mut rh = ui_px(140.0);
    let mut gap = ui_px(20.0);
    // A list longer than the screen has room for gets shorter rows, never below a 10 mm touch target,
    // then narrower gaps between them (sideways the Buttons sheet's Tools: 12 rows).
    let room = a.h - 2 * ui_px(40.0) - ui_px(120.0) - ui_px(72.0);
    if n * (rh + gap) > room {
        rh = clampi(room / n - gap, ui_px(118.0), rh);
    }
    if n * (rh + gap) > room {
        gap = clampi(room / n - rh, ui_px(10.0), gap);
    }
    let card = sheet_rect(a.w, a.h, ui_px(120.0) + ui_px(72.0) + n * (rh + gap));
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let in_ = ui_sheet(c, card, title);
    for (i, row) in rows.iter().enumerate() {
        let r = Rect::new(in_.x, in_.y + i as i32 * (rh + gap), in_.w, rh);
        ui_frame(c, r, ui_px(16.0), if row.st == ButtonStyle::Selected { 0 } else { 255 }, true, 0);
        ui_frame(
            c,
            r,
            ui_px(16.0),
            if row.st == ButtonStyle::Disabled { 170 } else { 0 },
            false,
            if row.st == ButtonStyle::Primary { ui_px(6.0) } else { ui_px(3.0) },
        );
        let fg: u8 = if row.st == ButtonStyle::Selected {
            255
        } else if row.st == ButtonStyle::Disabled {
            170
        } else {
            0
        };
        if row.icon != IconKind::None {
            ui_icon(c, row.icon, Rect::new(r.x + ui_px(20.0), r.y + ui_px(15.0), rh - ui_px(30.0), rh - ui_px(30.0)), fg);
        }
        font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(r.x + rh + ui_px(10.0), r.y, r.w - rh - ui_px(30.0), r.h), &row.label, fg);
        if row.st != ButtonStyle::Disabled {
            hits_add(&mut s.hits, r, row.id, row.arg);
        }
    }
}

// The language Settings > Language has picked: its index, or -1 for automatic (language=auto,
// or a language this version does not speak).
fn language_picked(a: &App) -> i32 {
    if i18n_setting_known(a.settings.language.as_str()) {
        i18n_find(a.settings.language.as_str())
    } else {
        -1
    }
}

// "Automatic (English)": the choice that follows the Kindle, with what it stands for here.
fn language_auto_label(out: &mut String, cap: usize) {
    // TRANSLATORS: in Settings > Language, the choice that follows the Kindle's own language, with
    // that language's name (each language is named in itself: "English", "Español").
    csnprintf!(out, cap, tr("Automatic (%s)"), i18n_language_name(i18n_find(i18n_auto())).unwrap_or("(null)"));
}

fn draw_settings(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let mut lang = String::new();
    let mut lang_line = String::new();
    if language_picked(a) < 0 {
        language_auto_label(&mut lang, 96);
    } else {
        csnprintf!(&mut lang, 96, "%s", i18n_language_name(language_picked(a)).unwrap_or("(null)"));
    }
    let App { screen: c, lib, settings, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let card = sheet_rect(aw, ah, ui_px(1820.0));
    let in_ = ui_sheet(c, card, tr("Settings"));
    let rh = ui_px(140.0);
    let gap = ui_px(24.0);
    let mut y = in_.y;
    let r1 = Rect::new(in_.x, y, in_.w, rh);
    ui_button(c, r1, IconKind::None, "", ButtonStyle::Plain);
    ui_icon(c, IconKind::Tune, Rect::new(r1.x + ui_px(20.0), r1.y + ui_px(15.0), rh - ui_px(30.0), rh - ui_px(30.0)), 0);
    // TRANSLATORS: the rows of Settings: buttons with an icon (1470 px at 44 px in bold), then
    // lines a tap changes (1530 px at 44 px) and checkboxes (1548 px at 44 px, or two lines at
    // 34 px). Sideways they are 80 px wider.
    let pens = tr("Pen sensitivity (pressure + tilt per tool)");
    font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(r1.x + rh + ui_px(10.0), r1.y, r1.w - rh - ui_px(30.0), rh), pens, 0);
    hits_add(&mut s.hits, r1, H_SET_PENS, 0);
    y += rh + gap;
    let r2 = Rect::new(in_.x, y, in_.w, rh);
    ui_button(c, r2, IconKind::None, "", ButtonStyle::Plain);
    ui_icon(c, IconKind::Target, Rect::new(r2.x + ui_px(20.0), r2.y + ui_px(15.0), rh - ui_px(30.0), rh - ui_px(30.0)), 0);
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(r2.x + rh + ui_px(10.0), r2.y, r2.w - rh - ui_px(30.0), rh),
        tr("Calibrate pen and touch"),
        0,
    );
    hits_add(&mut s.hits, r2, H_SET_CALIB, 0);
    y += rh + gap;
    let r2b = Rect::new(in_.x, y, in_.w, rh);
    ui_button(c, r2b, IconKind::None, "", ButtonStyle::Plain);
    ui_icon(c, IconKind::Brightness, Rect::new(r2b.x + ui_px(20.0), r2b.y + ui_px(15.0), rh - ui_px(30.0), rh - ui_px(30.0)), 0);
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(r2b.x + rh + ui_px(10.0), r2b.y, r2b.w - rh - ui_px(30.0), rh),
        tr("Display: brightness, warmth, invert"),
        0,
    );
    hits_add(&mut s.hits, r2b, H_SET_DISPLAY, 0);
    y += rh + gap;
    let rt = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    canvas_fill(c, rt, 255);
    ui_icon(c, IconKind::Castle, Rect::new(rt.x, rt.y + ui_px(15.0), ui_px(80.0), ui_px(80.0)), 0);
    // TRANSLATORS: the style Grimoire is drawn in, and the other one a tap changes to.
    let mut style = tr("Style: Gothic (tap for Classic)");
    if ui_theme() == UiTheme::Classic {
        style = tr("Style: Classic (tap for Gothic)");
    }
    font_draw_left(c, FontFace::Regular, ui_text_size(), Rect::new(rt.x + ui_px(110.0), rt.y, rt.w - ui_px(110.0), rt.h), style, 0);
    hits_add(&mut s.hits, rt, H_SET_THEME, 0);
    y += ui_px(130.0);
    let r3 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    let mut row = String::new();
    // TRANSLATORS: %s is the folder on the Kindle's storage (seen over USB) that gets a PDF of
    // every notebook.
    csnprintf!(&mut row, 256, tr("Mirror PDFs to %s (USB)"), "grimoire/PDFs");
    ui_checkbox(c, r3, &row, settings.mirror_pdf);
    hits_add(&mut s.hits, r3, H_SET_MIRROR, 0);
    y += ui_px(130.0);
    let r4 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    // What holding the pen's side button does while writing (settings.ini side_button 0 to 3).
    static BTN_ROWS: [&str; 4] = [
        // TRANSLATORS: what holding the pen's button while writing does; a tap moves to the next.
        n_("Holding the pen button does nothing (tap to change)"),
        n_("Holding the pen button erases (tap to change)"),
        n_("Holding the pen button highlights (tap to change)"),
        // TRANSLATORS: the fourth choice of the same row: while the pen's button is held, the pen
        // draws white ink (which takes away the ink under it and keeps the paper's lines). 1530 px
        // at 44 px, sideways 1610, or two lines at 34 px.
        n_("Holding the pen button draws white (tap to change)"),
    ];
    canvas_fill(c, r4, 255);
    ui_icon(c, IconKind::Highlighter, Rect::new(r4.x, r4.y + ui_px(15.0), ui_px(80.0), ui_px(80.0)), 0);
    ui_text(
        c,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        Rect::new(r4.x + ui_px(110.0), r4.y, r4.w - ui_px(110.0), r4.h),
        tr(BTN_ROWS[clampi(settings.side_button, 0, 3) as usize]),
        2,
        false,
        0,
    );
    hits_add(&mut s.hits, r4, H_SET_SIDE, 0);
    y += ui_px(130.0);
    let r5 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    ui_checkbox(c, r5, tr("Turn with the Kindle (portrait and landscape)"), settings.follow_rotation);
    hits_add(&mut s.hits, r5, H_SET_ROTATE, 0);
    y += ui_px(130.0);
    let r6 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    ui_checkbox(c, r6, tr("Pinch with two fingers to zoom"), settings.pinch_zoom);
    hits_add(&mut s.hits, r6, H_SET_PINCH, 0);
    y += ui_px(130.0);
    // The last row the sheet has room for: sideways in Gothic 18 px are left above Done.
    let rl = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    // TRANSLATORS: a row of Settings, with the choice: "Automatic (English)", or a language's name.
    csnprintf!(&mut lang_line, 160, tr("Language: %s"), &lang);
    canvas_fill(c, rl, 255);
    ui_icon(c, IconKind::Language, Rect::new(rl.x, rl.y + ui_px(15.0), ui_px(80.0), ui_px(80.0)), 0);
    ui_text(
        c,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        Rect::new(rl.x + ui_px(110.0), rl.y, rl.w - ui_px(110.0), rl.h),
        &lang_line,
        2,
        false,
        0,
    );
    hits_add(&mut s.hits, rl, H_SET_LANGUAGE, 0);
    y += ui_px(150.0);
    let caps = input_caps();
    let mut pen = String::new();
    let mut about = String::new();
    let pressure = caps.has_pressure;
    let tilt = caps.has_tilt;
    if !caps.has_pen {
        // TRANSLATORS: Settings' last lines (34 px): after the version, the pen Grimoire found (its
        // device's name, %s) and what it reports; then where Grimoire keeps its data (%s).
        csnprintf!(&mut pen, 160, "%s", tr("pen: none"));
    } else if pressure && tilt {
        csnprintf!(&mut pen, 160, tr("pen: %s, pressure, tilt"), &caps.pen_name);
    } else if pressure {
        csnprintf!(&mut pen, 160, tr("pen: %s, pressure"), &caps.pen_name);
    } else if tilt {
        csnprintf!(&mut pen, 160, tr("pen: %s, tilt"), &caps.pen_name);
    } else {
        csnprintf!(&mut pen, 160, tr("pen: %s"), &caps.pen_name);
    }
    csnprintf!(&mut about, 320, "Grimoire %s  %s", GRIMOIRE_VERSION, &pen);
    font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(in_.x, y, in_.w, ui_px(60.0)), &about, 60);
    // (The data's path is a name: a long one is cut short, as names are. tests/test_i18n.c checks
    // that the label fits with the Kindle's path.)
    csnprintf!(&mut about, 320, tr("Data: %s"), &store_root());
    font_draw_left_name(c, FontFace::Regular, ui_small_size(), Rect::new(in_.x, y + ui_px(60.0), in_.w, ui_px(60.0)), &about, 60);
    let done = Rect::new(in_.x + in_.w - ui_px(380.0), in_.y + in_.h - ui_px(140.0), ui_px(380.0), ui_px(140.0));
    ui_button(c, done, IconKind::None, tr("Done"), ButtonStyle::Primary);
    hits_add(&mut s.hits, done, H_CANCEL, 0);
    let ko = Rect::new(in_.x, done.y, ui_px(420.0), done.h);
    ui_button(c, ko, IconKind::Notebook, "KOReader", ButtonStyle::Plain);
    hits_add(&mut s.hits, ko, H_KOREADER, 0);
    // Between KOReader and Done: Controller and Updates, each half the room.
    let mid = done.x - ko.x - ko.w - ui_px(90.0);
    let keys = Rect::new(ko.x + ko.w + ui_px(30.0), done.y, mid / 2, done.h);
    let upd = Rect::new(keys.x + keys.w + ui_px(30.0), done.y, mid - mid / 2, done.h);
    // TRANSLATORS: Settings' bottom buttons, each a word under its icon (about 360 px at 34 px):
    // to the Controller sheet (a controller's or keyboard's buttons as shortcuts), and to the
    // Updates sheet (check for a newer Grimoire and install it).
    ui_button(c, keys, IconKind::Gamepad, tr("Controller"), ButtonStyle::Plain);
    hits_add(&mut s.hits, keys, H_SHORTCUTS, 0);
    ui_button(c, upd, IconKind::Update, tr("Updates"), ButtonStyle::Plain);
    hits_add(&mut s.hits, upd, H_UPDATES, 0);
}

// The item MORE, MOVE and DELETE act on (S->target), if it is one.
fn target_item(s: &LibraryState) -> Option<Item> {
    if s.target >= 0 && (s.target as usize) < s.items.len() {
        Some(s.items[s.target as usize])
    } else {
        None
    }
}

// A notebook's or folder's name as a sheet's title shows it, its own copy: draw_menu takes the App,
// which holds the name.
fn name_label(name: &str) -> String {
    let mut label = String::new(); // (a name as the screens show it: ui_name_label)
    ui_name_label(name, &mut label, LABEL_CAP).to_string()
}

fn draw_modal(a: &mut App) {
    let it = target_item(lib(a));
    let modal = lib(a).modal;
    match modal {
        Modal::New => draw_new(a),
        Modal::Settings => draw_settings(a),
        Modal::Updates => {
            let mut status = String::new();
            let mut channel = String::new();
            let mut now = String::new();
            let avail = update_available();
            let s = lib(a);
            let (upd, newer, latest, why) = (s.upd, upd_newer(s), s.upd_latest.clone(), s.upd_why.clone());
            if !avail {
                // TRANSLATORS: the Updates sheet's first line (about 1100 px at 44 px in bold): the version
                // running; then while it checks; then what the check found: a newer one (%s), or that this
                // one (%s) is the newest; or why it could not check.
                csnprintf!(&mut status, 200, "%s", tr("Updates come through KPM, which is not here."));
            } else if upd == UPD_CHECKING {
                csnprintf!(&mut status, 200, "%s", tr("Checking for updates..."));
            } else if upd == UPD_DONE && newer {
                csnprintf!(&mut status, 200, tr("Grimoire %s is available."), &latest);
            } else if upd == UPD_DONE {
                csnprintf!(&mut status, 200, tr("Grimoire %s is the newest."), GRIMOIRE_VERSION);
            } else if upd == UPD_FAILED && why == "network" {
                csnprintf!(&mut status, 200, "%s", tr("Could not check. Is Wi-Fi on?"));
            } else if upd == UPD_FAILED && why == "no-kpm" {
                csnprintf!(&mut status, 200, "%s", tr("Updates come through KPM, which is not here."));
            } else if upd == UPD_FAILED {
                csnprintf!(&mut status, 200, "%s", tr("Could not check for updates."));
            } else {
                csnprintf!(&mut status, 200, tr("This is Grimoire %s."), GRIMOIRE_VERSION);
            }
            // TRANSLATORS: (Updates sheet) which updates: betas too (the preview channel), or releases
            // only (stable); a tap changes it.
            csnprintf!(&mut channel, 200, "%s", if upd_preview(a) { tr("Updates: betas and releases (tap for releases only)") }
                                                   else { tr("Updates: releases only (tap for betas too)") });
            // TRANSLATORS: (Updates sheet) installs the newer Grimoire (%s): Grimoire closes, KPM
            // installs it, and Grimoire starts again.
            csnprintf!(&mut now, 160, tr("Update to %s"), &latest);
            let can = avail && upd != UPD_CHECKING;
            let rows = [
                menu_row(IconKind::None, &status, H_NONE, 0, ButtonStyle::Disabled),
                menu_row(IconKind::None, &channel, H_UPD_CHANNEL, 0, if can { ButtonStyle::Plain } else { ButtonStyle::Disabled }),
                // TRANSLATORS: (Updates sheet) asks Grimoire's feed for a newer Grimoire (through KPM).
                menu_row(IconKind::Update, tr("Check for updates"), H_UPD_CHECK, 0, if can { ButtonStyle::Plain } else { ButtonStyle::Disabled }),
                menu_row(IconKind::Download, if newer { now.as_str() } else { tr("Update") }, H_UPD_NOW, 0, if newer { ButtonStyle::Primary } else { ButtonStyle::Disabled }),
                menu_row(IconKind::Close, tr("Done"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            // TRANSLATORS: the Updates sheet's title (Settings > Updates).
            draw_menu(a, tr("Updates"), &rows);
        }
        Modal::Pair => {
            let s = lib(a);
            let mut rows: Vec<MenuRow> = Vec::with_capacity(8);
            // TRANSLATORS: the Pair sheet's first line: what to do with the controller (each has its own
            // way: often a button held a few seconds).
            let first = if s.scanning { tr("Searching... Put the controller in pairing mode.") }
                        else if s.pairing { tr("Pairing...") } else { tr("Tap a controller to pair it.") };
            rows.push(menu_row(IconKind::None, first, H_NONE, 0, ButtonStyle::Disabled));
            let mut i = 0;
            while i < s.nfound && rows.len() < 5 {
                let d = &s.found[i as usize];
                let mut name = String::new();
                csnprintf!(&mut name, BT_NAME_LEN + 40, "%s", if !d.name.is_empty() { d.name.as_str() } else { d.address.as_str() });
                let chosen = s.pairing && d.address == s.pair_addr;
                let st = if chosen {
                    ButtonStyle::Selected
                } else if s.pairing {
                    ButtonStyle::Disabled
                } else {
                    ButtonStyle::Plain
                };
                rows.push(menu_row(IconKind::None, &name, H_PAIR_FOUND, i, st));
                i += 1;
            }
            let mut i = 0;
            while i < s.npaired && rows.len() < 6 {
                let p = &s.paired[i as usize];
                let mut name = String::new();
                // TRANSLATORS: (Pair sheet) unpairs a controller paired before; %s is its own name.
                csnprintf!(&mut name, BT_NAME_LEN + 40, tr("Forget %s"), if !p.name.is_empty() { p.name.as_str() } else { p.address.as_str() });
                rows.push(menu_row(IconKind::Trash, &name, H_PAIR_FORGET, i, if s.pairing { ButtonStyle::Disabled } else { ButtonStyle::Plain }));
                i += 1;
            }
            // TRANSLATORS: (Pair sheet) searches for controllers again.
            rows.push(menu_row(IconKind::None, tr("Search again"), H_PAIR_AGAIN, 0, if s.scanning || s.pairing { ButtonStyle::Disabled } else { ButtonStyle::Plain }));
            rows.push(menu_row(IconKind::Back, tr("Done"), H_BUTTONS_DONE, 0, ButtonStyle::Plain));
            // TRANSLATORS: the Pair sheet's title (Settings > Controller > Pair a controller).
            draw_menu(a, tr("Pair a controller"), &rows);
        }
        Modal::Buttons => {
            let mut rows: Vec<MenuRow> = Vec::with_capacity(16);
            let g = lib(a).buttons_group;
            if g < 0 {
                for (i, grp) in K_GROUPS.iter().enumerate() {
                    rows.push(menu_row(grp.icon, tr(grp.label), H_BUTTON_GROUP, i as i32, ButtonStyle::Plain));
                }
                let cp = controller_path();
                // TRANSLATORS: the Buttons sheet: forget the buttons learned (back to shortcuts.ini's).
                rows.push(menu_row(IconKind::Trash, tr("Reset buttons"), H_BUTTONS_RESET, 0, if std::fs::metadata(&cp).is_ok() { ButtonStyle::Plain } else { ButtonStyle::Disabled }));
                rows.push(menu_row(IconKind::Back, tr("Done"), H_BUTTONS_DONE, 0, ButtonStyle::Plain));
                // TRANSLATORS: the Buttons sheet's title (Settings > Controller > Buttons): a group of
                // actions, then a tap on a row and a press on the controller, makes that button do it.
                draw_menu(a, tr("Controller buttons"), &rows);
                return;
            }
            let hid_changes = a.hid_changes;
            lib(a).shown_hid_changes = hid_changes; // (pads_text names the buttons for the controller connected)
            let (learn_row, learn_step) = (lib(a).learn_row, lib(a).learn_step);
            for i in 0..group_rows(g) {
                let r = learn_row_at(g, i);
                let learning = learn_row == i;
                let mut text = String::new();
                if learning {
                    // TRANSLATORS: the Buttons sheet's row while it waits for a press (10 s): %s is the
                    // action's name as the rows show it ("Undo", "Pencil", "Next page"). About 1100 px at
                    // 44 px in bold.
                    csnprintf!(&mut text, 200, tr("Press a button for %s"), action_label(r.act[learn_step as usize]));
                } else {
                    let mut p1 = String::new();
                    let mut p2 = String::new();
                    pads_text(a, r.act[0], &mut p1, 64);
                    let label = match r.label {
                        Some(l) => tr(l),
                        None => action_label(r.act[0]),
                    };
                    if r.act[1] != 0 {
                        pads_text(a, r.act[1], &mut p2, 64);
                        csnprintf!(&mut text, 200, "%s: %s / %s", label, &p1, &p2);
                    } else {
                        csnprintf!(&mut text, 200, "%s: %s", label, &p1);
                    }
                }
                rows.push(menu_row(r.icon, &text, H_BUTTON_ROW, i, if learning { ButtonStyle::Selected } else { ButtonStyle::Plain }));
            }
            rows.push(menu_row(IconKind::Back, tr("Done"), H_BUTTONS_DONE, 0, ButtonStyle::Plain));
            draw_menu(a, tr(K_GROUPS[g as usize].title), &rows);
        }
        Modal::Shortcuts => {
            let mut count = String::new();
            let mut last = String::new();
            let mut status = String::new();
            // TRANSLATORS: (Controller sheet) how many shortcuts grimoire/shortcuts.ini (and controller.ini)
            // hold, and how many of their lines were refused.
            csnprintf!(&mut count, 160, tr("Mappings: %d; rejected: %d"), a.shortcuts.count, a.shortcuts.rejected);
            if a.last_shortcut_code != 0 {
                csnprintf!(
                    &mut last,
                    160,
                    tr("Last input: %s %d; modifiers: %d"),
                    if a.last_shortcut_gamepad { "pad" } else { "key" },
                    a.last_shortcut_code,
                    a.last_shortcut_mods
                );
            } else {
                csnprintf!(&mut last, 160, "%s", tr("Press a connected key or controller button"));
            }
            let (code, mods, gamepad) = (a.last_shortcut_code, a.last_shortcut_mods, a.last_shortcut_gamepad);
            let s = lib(a);
            s.shown_shortcut_code = code;
            s.shown_shortcut_mods = mods;
            s.shown_shortcut_gamepad = gamepad;
            hid_status(a, &mut status, 160);
            let bt_changes = a.bt_changes;
            lib(a).shown_bt_changes = bt_changes;
            let bs = bt_state();
            if bs == BtState::Starting {
                // TRANSLATORS: (Controller sheet) its status line while the Bluetooth helper starts, and when it
                // would not start.
                csnprintf!(&mut status, 160, "%s", tr("Bluetooth: starting..."));
            } else if bs == BtState::Failed {
                csnprintf!(&mut status, 160, "%s", tr("Bluetooth did not start (see grimoire.log)"));
            }
            let on = a.settings.shortcuts_enabled;
            let present = bt_present();
            let bt_up = bs == BtState::Owned || bs == BtState::External;
            let mut reload = String::new();
            // TRANSLATORS: (Controller sheet) reads grimoire/shortcuts.ini (and controller.ini) again: how
            // many shortcuts they hold, and how many of their lines were refused.
            csnprintf!(&mut reload, 160, tr("Reload shortcuts.ini (%d, %d refused)"), a.shortcuts.count, a.shortcuts.rejected);
            let rows = [
                menu_row(if on { IconKind::Check } else { IconKind::None }, tr("Enable shortcuts"), H_SHORTCUT_ENABLE, 0, ButtonStyle::Plain),
                // TRANSLATORS: (Controller sheet) starts the Bluetooth helper with Grimoire, for a
                // Bluetooth controller; without the helper unpacked in grimoire/bluetooth, the second.
                menu_row(
                    if a.settings.controller && present { IconKind::Check } else { IconKind::None },
                    if present { tr("Bluetooth controller") } else { tr("No Bluetooth helper in grimoire/bluetooth") },
                    H_BT_TOGGLE,
                    0,
                    if present && on { ButtonStyle::Plain } else { ButtonStyle::Disabled },
                ),
                // TRANSLATORS: (Controller sheet) opens the Pair sheet: find a Bluetooth controller and pair it.
                menu_row(IconKind::Plus, tr("Pair a controller..."), H_PAIR, 0, if bt_up && on { ButtonStyle::Plain } else { ButtonStyle::Disabled }),
                menu_row(IconKind::None, &status, H_NONE, 0, ButtonStyle::Disabled),
                // TRANSLATORS: (Controller sheet) opens the Buttons sheet: which controller button does what.
                menu_row(IconKind::None, tr("Buttons..."), H_BUTTONS, 0, if on { ButtonStyle::Plain } else { ButtonStyle::Disabled }),
                menu_row(IconKind::None, &reload, H_SHORTCUT_RELOAD, 0, ButtonStyle::Plain),
                menu_row(IconKind::None, &last, H_NONE, 0, ButtonStyle::Disabled),
                menu_row(IconKind::Close, tr("Done"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            let _ = count;
            // TRANSLATORS: the Controller sheet's title (Settings > Controller): a controller's or a
            // keyboard's buttons as shortcuts.
            draw_menu(a, tr("Controller and keys"), &rows);
        }
        Modal::Koreader => {
            let mut ko = String::new();
            let found = koreader_find(&mut ko, 800);
            let installed = koreader_item_installed(0);
            let if_found = if found { ButtonStyle::Plain } else { ButtonStyle::Disabled };
            let rows = [
                menu_row(IconKind::Notebook, if found { tr("Install / update Grimoire plugin") } else { tr("KOReader not found") }, H_KO_ITEM, 0, if_found),
                menu_row(IconKind::Trash, tr("Remove Grimoire plugin"), H_KO_ITEM, -1, if installed { ButtonStyle::Plain } else { ButtonStyle::Disabled }),
                menu_row(if koreader_item_installed(1) { IconKind::Check } else { IconKind::None }, tr("Disable diagonal screenshot gestures"), H_KO_ITEM, 1, if_found),
                menu_row(if koreader_item_installed(2) { IconKind::Check } else { IconKind::None }, tr("Square folder corners"), H_KO_ITEM, 2, if_found),
                menu_row(if koreader_item_installed(3) { IconKind::Check } else { IconKind::None }, tr("Terminess menu font"), H_KO_ITEM, 3, if_found),
                menu_row(IconKind::Close, tr("Done"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            draw_menu(a, "KOReader", &rows);
        }
        Modal::More => {
            let Some(it) = it else { return };
            let mut pdf = String::new();
            // TRANSLATORS: a notebook's menu, rows of 1470 px at 44 px in bold. %s is the folder on the
            // Kindle's storage the PDF goes in.
            csnprintf!(&mut pdf, 256, tr("Save PDF to %s now"), "grimoire/PDFs");
            let rows = [
                menu_row(IconKind::Notebook, tr("Open"), H_MORE_OPEN, 0, ButtonStyle::Plain),
                menu_row(IconKind::Rename, tr("Rename"), H_MORE_RENAME, 0, ButtonStyle::Plain),
                menu_row(IconKind::FolderMove, tr("Move to folder"), H_MORE_MOVE, 0, ButtonStyle::Plain),
                menu_row(IconKind::Download, &pdf, H_MORE_PDF, 0, ButtonStyle::Plain),
                menu_row(IconKind::Trash, tr("Delete"), H_MORE_DELETE, 0, ButtonStyle::Plain),
                menu_row(IconKind::Close, tr("Cancel"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            let title = name_label(lib(a).books[it.nb].name.as_str());
            draw_menu(a, &title, &rows);
        }
        Modal::FolderMore => {
            let Some(it) = it else { return };
            // TRANSLATORS: a folder's menu (1470 px at 44 px in bold); only an empty folder can go.
            let rows = [
                menu_row(IconKind::Rename, tr("Rename folder"), H_FMORE_RENAME, 0, ButtonStyle::Plain),
                menu_row(
                    IconKind::Trash,
                    if it.count != 0 { tr("Remove folder (move its notebooks out first)") } else { tr("Remove folder") },
                    H_FMORE_REMOVE,
                    0,
                    if it.count != 0 { ButtonStyle::Disabled } else { ButtonStyle::Plain },
                ),
                menu_row(IconKind::Close, tr("Cancel"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            let title = name_label(it.folder.as_str());
            draw_menu(a, &title, &rows);
        }
        Modal::Move => {
            let Some(it) = it else { return };
            let s = lib(a);
            let mut rows: Vec<MenuRow> = Vec::with_capacity(14);
            let curf = s.books[it.nb].folder;
            // TRANSLATORS: where a notebook can go: the Library itself, out of every folder (also in New
            // notebook: "In: Library (top level)").
            rows.push(menu_row(IconKind::Library, tr("Library (top level)"), H_MOVE_TOP, 0, if !curf.is_empty() { ButtonStyle::Plain } else { ButtonStyle::Selected }));
            let mut f = 0;
            while f < s.folders.len() && rows.len() < 12 {
                let st = if curf != s.folders[f].as_str() { ButtonStyle::Plain } else { ButtonStyle::Selected };
                rows.push(menu_row(IconKind::Folder, &s.folders[f], H_MOVE_TO, f as i32, st));
                f += 1;
            }
            rows.push(menu_row(IconKind::FolderPlus, tr("New folder..."), H_MOVE_NEW, 0, ButtonStyle::Plain));
            rows.push(menu_row(IconKind::Close, tr("Cancel"), H_CANCEL, 0, ButtonStyle::Plain));
            // TRANSLATORS: the title of the list of folders a notebook can be moved to.
            draw_menu(a, tr("Move to"), &rows);
        }
        Modal::Delete => {
            let Some(it) = it else { return };
            let mut title = String::new();
            let mut del = String::new();
            let name = name_label(lib(a).books[it.nb].name.as_str());
            // TRANSLATORS: the title of the question before a notebook is deleted, with its name.
            csnprintf!(&mut title, 256, tr("Delete \"%s\"?"), &name);
            // TRANSLATORS: the answers (1470 px at 44 px in bold): delete, into the trash folder on the
            // Kindle's storage (%s), or keep the notebook.
            csnprintf!(&mut del, 256, tr("Delete (moves it to %s)"), "grimoire/trash");
            let rows = [
                menu_row(IconKind::Trash, &del, H_DELETE_YES, 0, ButtonStyle::Primary),
                menu_row(IconKind::Close, tr("Keep it"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            draw_menu(a, &title, &rows);
        }
        Modal::Language => {
            // Automatic, then every language by its own name; a pick applies at once.
            let mut rows: Vec<MenuRow> = Vec::with_capacity(12);
            let cur = language_picked(a);
            let mut auto_label = String::new();
            language_auto_label(&mut auto_label, 96);
            rows.push(menu_row(IconKind::Language, &auto_label, H_LANG_PICK, -1, if cur < 0 { ButtonStyle::Selected } else { ButtonStyle::Plain }));
            let mut i = 0;
            while i < i18n_language_count() && rows.len() < 11 {
                let name = i18n_language_name(i).unwrap_or("(null)");
                rows.push(menu_row(IconKind::None, name, H_LANG_PICK, i, if cur == i { ButtonStyle::Selected } else { ButtonStyle::Plain }));
                i += 1;
            }
            rows.push(menu_row(IconKind::Close, tr("Cancel"), H_LANG_BACK, 0, ButtonStyle::Plain));
            // TRANSLATORS: the title of the list of languages (Settings > Language).
            draw_menu(a, tr("Language"), &rows);
        }
        Modal::Kb => {
            let (aw, ah) = (a.w, a.h);
            let App { screen, lib, .. } = a;
            let s = lib.as_deref_mut().unwrap();
            keyboard_draw(&mut s.kb, screen, Rect::new(0, 0, aw, ah));
        }
        Modal::Display => {
            let mut dm = std::mem::take(&mut lib(a).dm);
            dispmenu_draw(&mut dm, a);
            lib(a).dm = dm;
        }
        Modal::Music => {
            draw_new(a);
            let mut mb = std::mem::take(&mut lib(a).mb);
            musicbuild_draw(&mut mb, a);
            lib(a).mb = mb;
        }
        Modal::Templates => {
            draw_new(a);
            let mut ts = std::mem::take(&mut lib(a).ts);
            tplsheet_draw(&mut ts, a);
            lib(a).ts = ts;
        }
        Modal::None => {}
    }
}

// Where toasts must stay above while a sheet is open, so they never cover its controls: the
// keyboard's top; else, when a toast in its usual place would cover any of the sheet's controls
// (the buttons along a tall sheet's bottom), the top of a band of the toast's height clear of
// them, found going up from there. 0: the usual place is clear.
fn toast_floor(a: &App) -> i32 {
    let s = a.lib.as_deref().unwrap();
    if s.modal == Modal::None {
        return 0;
    }
    if s.modal == Modal::Kb {
        return s.kb.area.y;
    }
    // (The Custom music and Templates sheets lie over the New notebook sheet: only their own
    // controls count.)
    let l = if s.modal == Modal::Music {
        &s.mb.hits
    } else if s.modal == Modal::Templates {
        &s.ts.hits
    } else {
        &s.hits
    };
    // (A toast of two lines is taller: the band is as tall as the toast up now, else one line.)
    let t = ui_toast_rect(Rect::new(0, 0, a.w, a.h), if a.toast_visible { a.toast.as_str() } else { "" });
    let gap = ui_px(20.0);
    let mut y = t.y;
    let mut below = 0; // below: the control the toast must stay above
    let mut moved = true;
    while moved && y >= gap {
        moved = false;
        for h in &l.h {
            let r = h.r;
            if r.y < y + t.h && r.y + r.h > y {
                y = r.y - t.h - gap; // above this control, then look again from there
                below = r.y;
                moved = true;
            }
        }
    }
    below
}

fn redraw(a: &mut App, m: RefreshMode) {
    draw_base(a);
    if lib(a).modal != Modal::None {
        hits_clear(&mut lib(a).hits); // a modal captures all input
        draw_modal(a);
    }
    a.toast_floor = toast_floor(a);
    app_invalidate_all(a, m);
}

fn open_modal(a: &mut App, m: Modal) {
    lib(a).modal = m;
    redraw(a, RefreshMode::Ui);
}

fn close_modal(a: &mut App) {
    lib(a).modal = Modal::None;
    redraw(a, RefreshMode::Ui);
}

fn open_kb(a: &mut App, p: KbPurpose, title: &str, initial: &[u8]) {
    let s = lib(a);
    s.kbp = p;
    keyboard_open(&mut s.kb, title, initial);
    s.modal = Modal::Kb;
    redraw(a, RefreshMode::Ui);
}

// ---- actions ----

// Keeps grimoire/PDFs in step when a notebook is renamed, moved or deleted: the old mirror
// PDF goes away, and (unless deleting) a fresh one is written under the new name/folder.
fn mirror_follow(a: &mut App, id: &str, deleting: bool) {
    let mut now = Box::<NotebookInfo>::default();
    if store_load_info(id, &mut now) == 0 {
        let mirror_dir = app_mirror_dir();
        let (dpi, mirror_pdf) = (a.dpi, a.settings.mirror_pdf);
        let s = lib(a);
        for i in 0..s.books.len() {
            if s.books[i].id != id {
                continue;
            }
            let mut oldp = String::new();
            let mut newp = String::new();
            export_mirror_path(&s.books[i], &mirror_dir, &mut oldp, 1200);
            export_mirror_path(&now, &mirror_dir, &mut newp, 1200);
            let had = std::fs::metadata(&oldp).is_ok();
            if had && !deleting && oldp != newp {
                // A rename or move: move the PDF along (instant) instead of re-rendering it.
                let mut dir = String::new();
                strlcpy_q(&mut dir, &newp, 1200);
                if let Some(slash) = dir.rfind('/') {
                    dir.truncate(slash);
                }
                mkdir_p(&dir);
                if std::fs::rename(&oldp, &newp).is_err() && mirror_pdf {
                    let _ = std::fs::remove_file(&oldp);
                    bg_mirror_pdf(id, dpi, &mirror_dir, false);
                }
            } else if had && deleting {
                let _ = std::fs::remove_file(&oldp);
            }
        }
    }
}

// A notebook on a template (More papers...): templates.h. False if that could not be done: the
// notebook stays on the paper picked before.
fn start_on_template(nb: &mut NotebookInfo, path: &str) -> bool {
    let mut why: &'static str = "";
    if template_start_notebook(nb, path, &mut why) == 0 {
        return true;
    }
    qlog!("library: %s: the new notebook's template could not be used: %s", &nb.id, why);
    false
}

fn create_notebook(a: &mut App) {
    let mut nb = Box::<NotebookInfo>::default();
    let s = lib(a);
    if store_create(s.new_name.as_str(), s.cur_folder.as_str(), s.new_paper, &mut nb) == 0 {
        s.modal = Modal::None;
        let tpl_ok = s.new_tpl.is_empty() || start_on_template(&mut nb, s.new_tpl.as_str());
        let id = nb.id.as_str().to_string();
        drop(nb);
        editor_open_notebook(a, &id);
        if !tpl_ok {
            let mut msg = String::new();
            csnprintf!(&mut msg, 256, tr("Could not use that template (see %s)."), "grimoire.log");
            app_toast(a, &msg, 4000);
        }
        return;
    }
    drop(nb);
    app_toast(a, tr("Could not create the notebook (storage full?)"), 3000);
}

fn kb_finished(a: &mut App, ok: bool) {
    let s = lib(a);
    let txt = s.kb.text.as_str().to_string();
    let it = target_item(s);
    let kbp = s.kbp;
    let mut back = Modal::None;
    if ok && !txt.is_empty() {
        match kbp {
            KbPurpose::NewName => {
                lib(a).new_name.set(&txt);
                back = Modal::New;
            }
            KbPurpose::Rename => {
                if let Some(it) = it.filter(|it| !it.is_folder) {
                    let id = lib(a).books[it.nb].id.as_str().to_string();
                    store_rename(&id, &txt);
                    mirror_follow(a, &id, false);
                }
            }
            KbPurpose::NewFolder => {
                store_add_folder(&txt);
            }
            KbPurpose::MoveNewFolder => {
                store_add_folder(&txt);
                if let Some(it) = it.filter(|it| !it.is_folder) {
                    let id = lib(a).books[it.nb].id.as_str().to_string();
                    store_set_folder(&id, &txt);
                    mirror_follow(a, &id, false);
                }
            }
            KbPurpose::RenameFolder => {
                if let Some(it) = it.filter(|it| it.is_folder) {
                    for i in 0..lib(a).books.len() {
                        if lib(a).books[i].folder == it.folder {
                            let id = lib(a).books[i].id.as_str().to_string();
                            store_set_folder(&id, &txt);
                            mirror_follow(a, &id, false);
                        }
                    }
                    if store_remove_folder(it.folder.as_str()) == 0 {
                        export_mirror_prune_folder(it.folder.as_str(), &app_mirror_dir());
                    }
                    store_add_folder(&txt);
                }
            }
        }
    } else if kbp == KbPurpose::NewName {
        back = Modal::New;
    }
    lib(a).modal = back;
    reload(a);
    redraw(a, RefreshMode::Ui);
}

fn act(a: &mut App, h: &Hit) {
    let it = target_item(lib(a));
    match h.id {
        H_BACK => {
            let s = lib(a);
            s.cur_folder.clear();
            s.page = 0;
            reload(a);
            redraw(a, RefreshMode::Full);
        }
        H_NEW => {
            let mut base = FixedStr::<STORE_NAME_LEN>::new();
            let mut day = String::new();
            format_date_short(wall_time(), &mut day, 64);
            // TRANSLATORS: a new notebook's name until it is renamed, with the day ("Notebook Oct 10").
            base.set(&cformat!(tr("Notebook %s"), &day));
            let s = lib(a);
            let mut new_name = FixedStr::<STORE_NAME_LEN>::new();
            unique_name(s, &base, &mut new_name);
            s.new_name = new_name;
            s.new_paper = PAPER_MEDIUM;
            s.new_tpl.clear();
            canvas_free(&mut s.tpl_prev);
            open_modal(a, Modal::New);
        }
        // TRANSLATORS: the keyboard's label for the name being typed: a folder's, or a notebook's.
        H_NEWFOLDER => open_kb(a, KbPurpose::NewFolder, c_("keyboard", "Folder"), b""),
        H_SHARE => share_open(a),
        H_SETTINGS => open_modal(a, Modal::Settings),
        H_EXIT => a.running = false,
        H_PREV | H_NEXT => {
            let s = lib(a);
            // Only the visible page's thumbnails stay in memory (each is ~0.5 MB).
            for i in 0..s.books.len() {
                canvas_free(&mut s.thumbs[i]);
                s.thumb_tried[i] = false;
            }
            s.page += if h.id == H_NEXT { 1 } else { -1 };
            redraw(a, RefreshMode::Ui);
        }
        H_CARD => {
            let s = lib(a);
            if h.arg >= s.items.len() as i32 {
                return;
            }
            let item = s.items[h.arg as usize];
            if item.is_folder {
                s.cur_folder.set_bytes(item.folder.as_bytes());
                s.page = 0;
                reload(a);
                redraw(a, RefreshMode::Full);
            } else {
                let id = s.books[item.nb].id.as_str().to_string();
                editor_open_notebook(a, &id);
            }
        }
        H_CARD_MORE => {
            let s = lib(a);
            s.target = h.arg;
            let m = if s.items[h.arg as usize].is_folder { Modal::FolderMore } else { Modal::More };
            open_modal(a, m);
        }
        H_PAPER => {
            let s = lib(a);
            s.new_paper = PaperKind(h.arg as u32);
            s.new_tpl.clear();
            redraw(a, RefreshMode::Ui);
        }
        H_PAPER_CUSTOM => {
            let start = lib(a).new_paper;
            let mut mb = std::mem::take(&mut lib(a).mb);
            musicbuild_open(&mut mb, a, start);
            lib(a).mb = mb;
            open_modal(a, Modal::Music);
        }
        H_PAPER_MORE => {
            let current = lib(a).new_tpl.as_str().to_string();
            let mut ts = std::mem::take(&mut lib(a).ts);
            tplsheet_open(&mut ts, a, None, PAPER_BLANK, Some(&current));
            lib(a).ts = ts;
            open_modal(a, Modal::Templates);
        }
        H_NEW_RENAME => {
            let name = lib(a).new_name;
            open_kb(a, KbPurpose::NewName, c_("keyboard", "Name"), name.as_bytes());
        }
        H_NEW_CREATE => create_notebook(a),
        H_CANCEL => close_modal(a),
        H_SHORTCUTS => open_modal(a, Modal::Shortcuts),
        H_SHORTCUT_ENABLE | H_SHORTCUT_RELOAD => {
            let mut path = String::new();
            csnprintf!(&mut path, 1024, "%s/shortcuts.ini", &store_root());
            if h.id == H_SHORTCUT_ENABLE {
                if !a.settings.shortcuts_enabled && shortcuts_write_defaults(&path) != 0 {
                    app_toast(a, tr("Could not save shortcuts.ini"), 5000);
                    return;
                }
                a.settings.shortcuts_enabled = !a.settings.shortcuts_enabled;
                settings_save(&a.settings);
                qlog!("settings: shortcuts %d, controller %d", a.settings.shortcuts_enabled, a.settings.controller);
                input_shortcuts_enable(a.settings.shortcuts_enabled);
                // The Bluetooth helper serves shortcuts alone: it goes with them, and comes back with
                // them while Bluetooth controller stays on.
                if !a.settings.shortcuts_enabled {
                    bt_stop();
                } else if a.settings.controller && bt_present() {
                    bt_start();
                }
            }
            reload_shortcuts(a);
            redraw(a, RefreshMode::Ui);
        }
        H_BUTTONS => {
            let s = lib(a);
            s.buttons_group = -1;
            s.learn_row = -1;
            open_modal(a, Modal::Buttons);
        }
        H_BUTTON_GROUP => {
            let s = lib(a);
            s.buttons_group = clampi(h.arg, 0, N_GROUPS - 1);
            s.learn_row = -1;
            redraw(a, RefreshMode::Ui);
        }
        H_BUTTON_ROW => {
            let s = lib(a);
            s.learn_row = if s.learn_row == h.arg { -1 } else { h.arg }; // (a second tap stops it)
            s.learn_step = 0;
            s.learn_at = now_ms();
            redraw(a, RefreshMode::Ui);
        }
        H_BUTTONS_RESET => {
            let cp = controller_path();
            let _ = std::fs::remove_file(&cp);
            lib(a).learn_row = -1;
            reload_shortcuts(a);
            redraw(a, RefreshMode::Ui);
        }
        H_BUTTONS_DONE => {
            let s = lib(a);
            s.learn_row = -1;
            if s.modal == Modal::Buttons && s.buttons_group >= 0 {
                // a group: back to the list
                s.buttons_group = -1;
                redraw(a, RefreshMode::Ui);
                return;
            }
            s.bt_next = ASK_NONE;
            open_modal(a, Modal::Shortcuts);
        }
        H_UPDATES => {
            let s = lib(a);
            if s.upd != UPD_CHECKING {
                s.upd = UPD_IDLE;
            }
            open_modal(a, Modal::Updates);
        }
        H_UPD_CHANNEL => {
            a.settings.updates = if upd_preview(a) { 1 } else { 2 };
            settings_save(&a.settings);
            lib(a).upd = UPD_IDLE; // (what a check found was the other channel's)
            redraw(a, RefreshMode::Ui);
        }
        H_UPD_CHECK => {
            if update_check(upd_preview(a)) {
                lib(a).upd = UPD_CHECKING;
            }
            redraw(a, RefreshMode::Ui);
        }
        H_UPD_NOW => {
            if !upd_newer(lib(a)) {
                return;
            }
            if update_request(upd_preview(a)) != 0 {
                // TRANSLATORS: a toast: the update could not be asked for (the storage is full, say).
                app_toast(a, tr("Could not start the update"), 3000);
                return;
            }
            qlog!("update: leaving so KPM can install %s", &lib(a).upd_latest);
            app_leave_for_update(a);
        }
        H_BT_TOGGLE => {
            a.settings.controller = !a.settings.controller;
            settings_save(&a.settings);
            qlog!("settings: shortcuts %d, controller %d", a.settings.shortcuts_enabled, a.settings.controller);
            if a.settings.controller {
                bt_start();
            } else {
                bt_stop();
            }
            redraw(a, RefreshMode::Ui);
        }
        H_PAIR => {
            pair_open(lib(a));
            redraw(a, RefreshMode::Ui);
        }
        H_PAIR_AGAIN => {
            pair_open(lib(a));
            redraw(a, RefreshMode::Ui);
        }
        H_PAIR_FOUND => {
            let s = lib(a);
            if s.pairing || h.arg < 0 || h.arg >= s.nfound {
                return;
            }
            let addr = s.found[h.arg as usize].address;
            s.pair_addr.set_bytes(addr.as_bytes());
            s.pairing = true;
            s.scanning = false;
            s.pair_until = now_ms().wrapping_add(PAIR_MS);
            bt_ask(s, ASK_PAIR, 0);
            redraw(a, RefreshMode::Ui);
        }
        H_PAIR_FORGET => {
            let s = lib(a);
            if s.pairing || h.arg < 0 || h.arg >= s.npaired {
                return;
            }
            let addr = s.paired[h.arg as usize].address;
            s.pair_addr.set_bytes(addr.as_bytes());
            bt_ask(s, ASK_REMOVE, 0);
        }
        H_KOREADER => open_modal(a, Modal::Koreader),
        H_KO_ITEM => {
            let item = if h.arg < 0 { 0 } else { h.arg };
            let enabled = if h.arg < 0 { false } else { item == 0 || !koreader_item_installed(item) };
            if koreader_item_set(item, enabled) == 0 {
                app_toast(a, tr("Updated. Restart KOReader to apply."), 5000);
            } else {
                app_toast(a, tr("Could not update KOReader. A different patch or a storage error?"), 7000);
            }
            redraw(a, RefreshMode::Ui);
        }
        H_SET_PENS => {
            lib(a).modal = Modal::None;
            let tool = a.settings.current_tool;
            pensettings_open(a, tool);
        }
        H_SET_CALIB => {
            lib(a).modal = Modal::None;
            calibrate_open(a);
        }
        H_SET_MIRROR => {
            a.settings.mirror_pdf = !a.settings.mirror_pdf;
            settings_save(&a.settings);
            redraw(a, RefreshMode::Ui);
        }
        H_SET_PINCH => {
            a.settings.pinch_zoom = !a.settings.pinch_zoom;
            settings_save(&a.settings);
            redraw(a, RefreshMode::Ui);
        }
        H_SET_THEME => {
            a.settings.theme = if a.settings.theme == UiTheme::Classic as i32 { UiTheme::Gothic as i32 } else { UiTheme::Classic as i32 };
            ui_set_theme(if a.settings.theme == UiTheme::Classic as i32 { UiTheme::Classic } else { UiTheme::Gothic });
            settings_save(&a.settings);
            redraw(a, RefreshMode::Full);
        }
        H_SET_DISPLAY => {
            let mut dm = std::mem::take(&mut lib(a).dm);
            dispmenu_open(&mut dm, a);
            lib(a).dm = dm;
            open_modal(a, Modal::Display);
        }
        H_SET_ROTATE => {
            a.settings.follow_rotation = !a.settings.follow_rotation;
            settings_save(&a.settings);
            if !a.settings.follow_rotation && a.rotation != 0 {
                app_set_rotation(a, 0); // back to portrait and stay there
                return;
            }
            if a.settings.follow_rotation && input_caps().has_orientation {
                a.pending_rotation = input_caps().orientation;
            }
            redraw(a, RefreshMode::Ui);
        }
        H_SET_SIDE => {
            a.settings.side_button = (clampi(a.settings.side_button, 0, 3) + 1) % 4;
            settings_save(&a.settings);
            redraw(a, RefreshMode::Ui);
        }
        H_SET_LANGUAGE => open_modal(a, Modal::Language),
        H_LANG_PICK => {
            // Saved and applied at once: the Settings sheet comes back in the language picked.
            let code = if h.arg >= 0 { i18n_language_code(h.arg) } else { Some("auto") };
            if let Some(code) = code.filter(|code| a.settings.language != *code) {
                a.settings.language.set(code);
                settings_save(&a.settings);
                i18n_apply(code);
            }
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Full);
        }
        H_LANG_BACK => {
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        H_MORE_OPEN => {
            if let Some(it) = it {
                let s = lib(a);
                let id = s.books[it.nb].id.as_str().to_string();
                s.modal = Modal::None;
                editor_open_notebook(a, &id);
            }
        }
        H_MORE_RENAME => {
            if let Some(it) = it {
                let name = lib(a).books[it.nb].name;
                open_kb(a, KbPurpose::Rename, c_("keyboard", "Name"), name.as_bytes());
            }
        }
        H_MORE_MOVE => open_modal(a, Modal::Move),
        H_MORE_PDF => {
            if let Some(it) = it {
                let id = lib(a).books[it.nb].id.as_str().to_string();
                bg_mirror_pdf(&id, a.dpi, &app_mirror_dir(), true);
                lib(a).modal = Modal::None;
                redraw(a, RefreshMode::Ui);
                app_toast(a, tr("Saving the PDF in the background..."), 2500);
            }
        }
        H_MORE_DELETE => open_modal(a, Modal::Delete),
        H_DELETE_YES => {
            if let Some(it) = it.filter(|it| !it.is_folder) {
                let mut nb_path = String::new();
                let s = lib(a);
                export_mirror_path(&s.books[it.nb], &app_mirror_dir(), &mut nb_path, 1200);
                let _ = std::fs::remove_file(&nb_path); // the notebook goes to trash; its mirror PDF should not linger
                // A compaction of it, queued or running, stops first: it would write into the folder
                // in the trash (or leave a temporary file there).
                let id = s.books[it.nb].id.as_str().to_string();
                bg_cancel_notebook(&id);
                store_delete(&id);
            }
            lib(a).modal = Modal::None;
            reload(a);
            redraw(a, RefreshMode::Full);
            let mut msg = String::new();
            // TRANSLATORS: a deleted notebook went to the trash folder on the Kindle's storage (%s).
            csnprintf!(&mut msg, 256, tr("Moved to %s"), "grimoire/trash");
            app_toast(a, &msg, 2500);
        }
        H_FMORE_RENAME => {
            if let Some(it) = it {
                open_kb(a, KbPurpose::RenameFolder, c_("keyboard", "Folder"), it.folder.as_bytes());
            }
        }
        H_FMORE_REMOVE => {
            if let Some(it) = it {
                if store_remove_folder(it.folder.as_str()) == 0 {
                    export_mirror_prune_folder(it.folder.as_str(), &app_mirror_dir());
                }
            }
            lib(a).modal = Modal::None;
            reload(a);
            redraw(a, RefreshMode::Ui);
        }
        H_MOVE_TOP | H_MOVE_TO => {
            if let Some(it) = it.filter(|it| !it.is_folder) {
                let s = lib(a);
                let id = s.books[it.nb].id.as_str().to_string();
                let folder = if h.id == H_MOVE_TOP { String::new() } else { s.folders[h.arg as usize].clone() };
                store_set_folder(&id, &folder);
                mirror_follow(a, &id, false);
            }
            lib(a).modal = Modal::None;
            reload(a);
            redraw(a, RefreshMode::Ui);
        }
        H_MOVE_NEW => open_kb(a, KbPurpose::MoveNewFolder, c_("keyboard", "Folder"), b""),
        _ => {}
    }
}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn library_init(a: &mut App) {
    a.lib = Some(Box::default());
    let s = lib(a);
    s.target = -1;
    s.learn_row = -1;
    s.buttons_group = -1;
}

pub fn library_free(a: &mut App) {
    let Some(s) = a.lib.as_deref_mut() else { return };
    free_lists(s);
    for k in 0..PAPER_COUNT as usize {
        canvas_free(&mut s.paper_prev[k]);
    }
    canvas_free(&mut s.custom_prev);
    musicbuild_free(&mut s.mb);
    tplsheet_close(&mut s.ts);
    canvas_free(&mut s.tpl_prev);
    a.lib = None;
}

pub fn library_enter(a: &mut App) {
    // A turn, or the redraw after waking, re-lays out the same screen: the open sheet stays open
    // (every sheet lays itself out when drawn).
    let relayout = a.relayout;
    let s = lib(a);
    if !relayout {
        s.modal = Modal::None;
    }
    s.pressed = None;
    reload(a);
    redraw(a, RefreshMode::Full);
}

pub fn library_leave(a: &mut App) {
    lib(a).pressed = None;
}

/// Whether the Library has a control at (x, y): a button, a card, or any sheet that is up (a sheet
/// takes every tap, a tap outside it included). The clock and battery corner leaves such a tap to it.
pub fn library_control_at(a: &App, x: f32, y: f32) -> bool {
    a.lib.as_deref().is_none_or(|s| s.modal != Modal::None || hits_find(&s.hits, x, y).is_some())
}

pub fn library_event(a: &mut App, ev: &InEvent) {
    let ph = app_pointer_phase(ev);
    if ph < 0 {
        return;
    }
    let modal = lib(a).modal;
    if modal == Modal::Templates {
        let mut d = Rect::default();
        let mut ts = std::mem::take(&mut lib(a).ts);
        let r = tplsheet_pointer(&mut ts, a, ph, ev.x, ev.y, ev.t_ms, &mut d);
        if r == TsResult::Picked {
            // The notebook is made on it (create_notebook); its tile shows its preview meanwhile.
            let path = tplsheet_path(&ts).to_string();
            let s = lib(a);
            s.new_tpl.set(&path);
            canvas_free(&mut s.tpl_prev);
            if let Some(pv) = tplsheet_preview(&mut ts) {
                if canvas_init(&mut s.tpl_prev, pv.w, pv.h) == 0 {
                    canvas_blit(&mut s.tpl_prev, pv, Rect::new(0, 0, pv.w, pv.h), 0, 0);
                }
            }
            tplsheet_close(&mut ts);
        }
        lib(a).ts = ts;
        if r == TsResult::Picked || r == TsResult::Back || r == TsResult::Cancel {
            lib(a).modal = Modal::New; // back to New notebook, also from a tap outside the sheet
            redraw(a, RefreshMode::Ui);
        } else if !rect_is_empty(d) {
            app_invalidate(a, d, RefreshMode::Ui);
        }
        return;
    }
    if modal == Modal::Display {
        let mut d = Rect::default();
        let mut dm = std::mem::take(&mut lib(a).dm);
        let r = dispmenu_pointer(&mut dm, a, ph, ev.x, ev.y, ev.t_ms, &mut d);
        lib(a).dm = dm;
        if r == DmResult::Closed {
            lib(a).modal = Modal::Settings; // back to the settings sheet it was opened from
            redraw(a, RefreshMode::Ui);
        } else if r == DmResult::Inverted {
            redraw(a, RefreshMode::Full);
        } else if !rect_is_empty(d) {
            app_invalidate(a, d, if ph == 2 { RefreshMode::Ui } else { RefreshMode::Fast });
        }
        return;
    }
    if modal == Modal::Music {
        let mut d = Rect::default();
        let mut mb = std::mem::take(&mut lib(a).mb);
        let r = musicbuild_pointer(&mut mb, a, ph, ev.x, ev.y, ev.t_ms, &mut d);
        if r == MbResult::Done || r == MbResult::Cancel {
            if r == MbResult::Done {
                let s = lib(a);
                s.new_paper = musicbuild_result(&mb);
                s.new_tpl.clear(); // (a layout built takes the place of a template picked before)
            }
            musicbuild_free(&mut mb);
            lib(a).mb = mb;
            lib(a).modal = Modal::New;
            redraw(a, RefreshMode::Ui);
        } else {
            lib(a).mb = mb;
            if !rect_is_empty(d) {
                app_invalidate(a, d, RefreshMode::Ui);
            }
        }
        return;
    }
    if modal == Modal::Kb {
        let mut kb = std::mem::take(&mut lib(a).kb);
        let r = app_keyboard_pointer(a, &mut kb, ph, ev); // (what a key changed, drawn and refreshed)
        lib(a).kb = kb;
        if r == KbResult::Done || r == KbResult::Cancel {
            kb_finished(a, r == KbResult::Done);
        }
        return;
    }
    if ph == 0 {
        let s = lib(a);
        press_begin(&mut s.press, ev.x, ev.y, ev.t_ms);
        s.pressed_at = now_ms();
        let h = hits_find(&s.hits, ev.x, ev.y).copied();
        s.pressed = None;
        if let Some(h) = h {
            s.pressed = Some(h);
            app_press_feedback(a, h.r);
        }
    } else if ph == 1 {
        press_move(&mut lib(a).press, ev.x, ev.y);
    } else if let Some(h) = lib(a).pressed {
        lib(a).pressed = None;
        canvas_invert(&mut a.screen, h.r); // undo the pressed look
        app_invalidate(a, h.r, RefreshMode::Ui);
        let s = lib(a);
        press_move(&mut s.press, ev.x, ev.y);
        if press_is_tap(&s.press, h.r) || rect_contains(h.r, ev.x as i32, ev.y as i32) {
            act(a, &h);
        }
    }
}

pub fn library_tick(a: &mut App, now: u32) {
    let s = a.lib.as_deref().unwrap();
    if s.modal == Modal::Shortcuts
        && (s.shown_shortcut_code != a.last_shortcut_code
            || s.shown_shortcut_mods != a.last_shortcut_mods
            || s.shown_shortcut_gamepad != a.last_shortcut_gamepad
            || s.shown_hid_changes != a.hid_changes)
    {
        redraw(a, RefreshMode::Ui);
    }
    // A group's rows name the buttons as the controller connected labels them.
    let s = a.lib.as_deref().unwrap();
    if s.modal == Modal::Buttons && s.buttons_group >= 0 && s.shown_hid_changes != a.hid_changes {
        redraw(a, RefreshMode::Ui);
    }
    if lib(a).modal == Modal::Pair {
        bt_send(lib(a));
    }
    upd_poll(a);
    let s = a.lib.as_deref().unwrap();
    if s.modal == Modal::Shortcuts && s.shown_bt_changes != a.bt_changes {
        redraw(a, RefreshMode::Ui);
    }
    let s = lib(a);
    if s.modal == Modal::Buttons && s.learn_row >= 0 && now.wrapping_sub(s.learn_at) >= LEARN_MS {
        s.learn_row = -1; // no press in 10 s: as it was
        redraw(a, RefreshMode::Ui);
    }
    // A finger held still on a Buttons row: its buttons go, and nothing is pressed any more (the
    // release does nothing; the redraw took the pressed look away).
    let s = lib(a);
    if s.modal == Modal::Buttons
        && s.buttons_group >= 0
        && s.pressed.is_some_and(|p| p.id == H_BUTTON_ROW)
        && !s.press.moved
        && now.wrapping_sub(s.pressed_at) >= ROW_HOLD_MS
    {
        let row = s.pressed.map_or(0, |p| p.arg);
        s.pressed = None;
        forget_row(a, row);
    }
    if lib(a).modal == Modal::Kb {
        let mut kb = std::mem::take(&mut lib(a).kb);
        app_keyboard_tick(a, &mut kb, now);
        lib(a).kb = kb;
    }
    let mut d = Rect::default();
    if lib(a).modal == Modal::Templates {
        let mut ts = std::mem::take(&mut lib(a).ts);
        let drawn = tplsheet_tick(&mut ts, a, now, &mut d);
        lib(a).ts = ts;
        if drawn {
            app_invalidate(a, d, RefreshMode::Ui);
        }
    }
    // A background job ended (bgimage.h): the cards it may have changed are read again. A
    // template's picture made for a notebook since may be on its first page, whose card the job drew
    // again (bgwork_template.c: its thumb.pgm is another file now), or a card drawn from a
    // notebook's own picture (picture_card) drew it white meanwhile. Any other job (the templates'
    // previews) changes no card, and the Library is not drawn again for it.
    let s = lib(a);
    if s.card_gen != bgimage_generation() {
        s.card_gen = bgimage_generation();
        let mut changed = false;
        for i in 0..s.books.len().min(s.card_stamp.len()) {
            if !s.thumb_tried[i] || s.card_stamp[i] == store_thumb_stamp(s.books[i].id.as_str()) {
                continue;
            }
            canvas_free(&mut s.thumbs[i]);
            s.thumb_tried[i] = false;
            changed = true;
        }
        if changed && s.modal == Modal::None {
            s.cards_redrawn += 1;
            redraw(a, RefreshMode::Ui);
        }
    }
}

// ---- headless checks ----

/// strtol(s, &end, 10) with the target's long: the value and where it stopped (0 when no digits).
fn strtol10(s: &[u8]) -> (i64, usize) {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let ds = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == ds {
        return (0, 0);
    }
    let mut mag: i128 = 0;
    for &d in &s[ds..i] {
        mag = (mag * 10 + (d - b'0') as i128).min(1i128 << 80); // (past any long: saturates below)
    }
    let v = if neg { -mag } else { mag };
    (v.clamp(libc::c_long::MIN as i128, libc::c_long::MAX as i128) as i64, i)
}

// card.N (from 1, as library.items counts them): what card N shows, "folder", "thumb" (its
// thumb.pgm), "picture" (its own picture paper, picture_card) or "paper K" (the preview of paper
// K); card.N.luma: the mean of the pixels it shows, 0 to 255 ("none" for a folder); card.N.name: the
// name under it, as drawn (ui_name_label).
fn card_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    let (n, end) = strtol10(key.as_bytes());
    let rest = &key[end..];
    let luma = rest == ".luma";
    let name = rest == ".name";
    let dpi = a.dpi;
    let s = lib(a);
    if n < 1 || n > s.items.len() as i64 || (!rest.is_empty() && !luma && !name) {
        return -1;
    }
    let it = s.items[(n - 1) as usize];
    if name {
        // the name under the card, as drawn (ui_name_label)
        let mut label = String::new();
        csnprintf!(out, cap, "%s", ui_name_label(if it.is_folder { it.folder.as_str() } else { s.books[it.nb].name.as_str() }, &mut label, LABEL_CAP));
        return 0;
    }
    if it.is_folder {
        csnprintf!(out, cap, "%s", if luma { "none" } else { "folder" });
        return 0;
    }
    let k = s.books[it.nb].paper;
    let t = thumb_for(s, dpi, it.nb);
    if luma {
        let mut sum: u64 = 0;
        if let Some(t) = t {
            for y in 0..t.h {
                for x in 0..t.w {
                    sum += t.px[y as usize * t.stride as usize + x as usize] as u64;
                }
            }
        }
        let np: u64 = t.map_or(0, |t| t.w as u64 * t.h as u64);
        csnprintf!(out, cap, "%d", (sum + np / 2).checked_div(np).map_or(255, |m| m as i32));
    } else if !s.thumbs[it.nb].px.is_empty() {
        csnprintf!(out, cap, "%s", if s.thumb_picture[it.nb] { "picture" } else { "thumb" });
    } else {
        // (as paper_preview draws it)
        csnprintf!(out, cap, "paper %u", if !paper_is_known(k) || paper_is_image(k) { PAPER_BLANK.0 } else { k.0 });
    }
    0
}

/// The headless runner's "library.*" keys, asked without the prefix (app_probe's contract);
/// "templates.*" keys reach it whole (docs/SIM.md).
// The script runner's "library.*" keys (app_probe forwards them without the prefix).
pub fn library_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    static SHEETS: [&str; 17] = [
        "none",        // MOD_NONE
        "new",         // MOD_NEW
        "settings",    // MOD_SETTINGS
        "more",        // MOD_MORE
        "folder-more", // MOD_FOLDER_MORE
        "move",        // MOD_MOVE
        "delete",      // MOD_DELETE
        "keyboard",    // MOD_KB
        "display",     // MOD_DISPLAY
        "music",       // MOD_MUSIC
        "language",    // MOD_LANGUAGE
        "templates",   // MOD_TEMPLATES
        "koreader",    // MOD_KOREADER
        "shortcuts",   // MOD_SHORTCUTS
        "buttons",     // MOD_BUTTONS
        "pair",        // MOD_PAIR
        "updates",     // MOD_UPDATES
    ];
    if a.lib.is_none() {
        return -1;
    }
    let pp = per_page(a);
    let s = lib(a);
    if key == "sheet" {
        match SHEETS.get(s.modal as usize) {
            Some(m) => csnprintf!(out, cap, "%s", *m),
            None => csnprintf!(out, cap, "%d", s.modal as i32),
        };
    } else if key == "learning" {
        // The action the Buttons sheet waits for a press for ("undo", "next-page"), else empty.
        let n = if s.modal == Modal::Buttons && s.buttons_group >= 0 && s.learn_row >= 0 {
            shortcut_action_name(learning_action(s))
        } else {
            None
        };
        csnprintf!(out, cap, "%s", n.unwrap_or(""));
    } else if let Some(action) = key.strip_prefix("buttons.") {
        // The Buttons sheet's buttons for an action ("buttons.undo"): "B, Up", "none".
        pads_text(a, shortcut_action(action), out, cap);
    } else if key == "buttons_group" {
        // The Buttons sheet's group open ("editing", "tools", "pages", "menus"), else empty.
        let open = s.modal == Modal::Buttons && s.buttons_group >= 0;
        csnprintf!(out, cap, "%s", if open { K_GROUPS[s.buttons_group as usize].probe } else { "" });
    } else if key == "update" {
        // The Updates sheet's check: "idle", "checking", "newer <version>", "newest <version>" or
        // "failed <why>".
        if s.upd == UPD_CHECKING {
            csnprintf!(out, cap, "checking");
        } else if s.upd == UPD_DONE {
            csnprintf!(out, cap, "%s %s", if upd_newer(s) { "newer" } else { "newest" }, &s.upd_latest);
        } else if s.upd == UPD_FAILED {
            csnprintf!(out, cap, "failed %s", &s.upd_why);
        } else {
            csnprintf!(out, cap, "idle");
        }
    } else if key == "pair" {
        // The Pair sheet: "scanning", "pairing" or "idle", then the controllers found and paired.
        csnprintf!(out, cap, "%s %d %d", if s.scanning { "scanning" } else if s.pairing { "pairing" } else { "idle" }, s.nfound, s.npaired);
    } else if key == "bluetooth" {
        static NAMES: [&str; 6] = ["stopped", "starting", "owned", "external", "failed", "stopping"];
        csnprintf!(out, cap, "%s", NAMES[bt_state() as usize]);
    } else if key == "folder" {
        csnprintf!(out, cap, "%s", &s.cur_folder);
    } else if key == "items" {
        csnprintf!(out, cap, "%d", s.items.len() as i32);
    } else if key == "page" {
        csnprintf!(out, cap, "%d", s.page + 1);
    } else if key == "pages" {
        let pages = (s.items.len() as i32 + pp - 1) / pp;
        csnprintf!(out, cap, "%d", if pages > 0 { pages } else { 1 });
    } else if key == "text" {
        csnprintf!(out, cap, "%s", if s.modal == Modal::Kb { s.kb.text.as_str() } else { "" });
    } else if key == "hint" {
        csnprintf!(out, cap, "%d", if empty_hint_shown(s) { 1 } else { 0 });
    } else if key == "cards_redrawn" {
        csnprintf!(out, cap, "%d", s.cards_redrawn); // after background jobs that changed a card
    } else if key == "paper" {
        if !s.new_tpl.is_empty() {
            csnprintf!(out, cap, "template");
        } else if paper_is_custom(s.new_paper) {
            csnprintf!(out, cap, "custom");
        } else {
            csnprintf!(out, cap, "%d", s.new_paper.0 as i32);
        }
    } else if key == "template" {
        // The template New notebook makes its notebook on: its file's name without its extension.
        let tpl = s.new_tpl.as_str();
        let name = match tpl.rfind('/') {
            Some(slash) => &tpl[slash + 1..],
            None => tpl,
        };
        let len = name.rfind('.').unwrap_or(name.len());
        csnprintf!(out, cap, "%s", &name[..len]);
    } else if let Some(rest) = key.strip_prefix("templates.") {
        // (The C passes NULL for a closed sheet: one that is not open answers the same.)
        let closed = TplSheet::default();
        return tplsheet_probe(if s.modal == Modal::Templates { &s.ts } else { &closed }, rest, out, cap);
    } else if key == "music" {
        // The layout on the Custom music sheet while it is open, else New notebook's custom paper.
        let mut m = MusicLayout::default();
        if s.modal == Modal::Music {
            app_probe_music(Some(&s.mb.m), out, cap);
        } else {
            app_probe_music(if music_decode(s.new_paper, &mut m) { Some(&m) } else { None }, out, cap);
        }
    } else if let Some(rest) = key.strip_prefix("card.") {
        return card_probe(a, rest, out, cap);
    } else {
        return -1;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strtol10_reads_as_strtol() {
        assert_eq!(strtol10(b"3.luma"), (3, 1));
        assert_eq!(strtol10(b"12"), (12, 2));
        assert_eq!(strtol10(b" +7.name"), (7, 3));
        assert_eq!(strtol10(b"-2"), (-2, 2));
        assert_eq!(strtol10(b".luma"), (0, 0));
        assert_eq!(strtol10(b"x"), (0, 0));
        assert_eq!(strtol10(b"99999999999999999999999").0, libc::c_long::MAX as i64);
    }

    #[test]
    fn the_buttons_groups_rows() {
        // The Tools group: the pens in the picker's order, then its own two rows.
        assert!(is_tools(1) && !is_tools(0) && !is_tools(2) && !is_tools(3));
        assert_eq!(group_rows(0), 7);
        assert_eq!(group_rows(1), TOOL_PENS as i32 + 2);
        assert_eq!(group_rows(2), 8);
        // Menus: the editor's menus, then the clock and battery corner (Rust only), last
        assert_eq!(group_rows(3), 8);
        assert_eq!(learn_row_at(3, 7).act, [SHORTCUT_STATUS, 0]);
        assert_eq!(action_label(SHORTCUT_STATUS), "Clock and battery");
        let first = learn_row_at(1, 0);
        assert_eq!(first.act, [SHORTCUT_TOOL_BASE + TOOL_ORDER[0].0, 0]);
        assert_eq!(learn_row_at(1, TOOL_PENS as i32).act, [SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL]);
        assert_eq!(learn_row_at(2, 0).label, Some("Previous / next page"));
        // Every action a row learns has a name of its own.
        for g in 0..N_GROUPS {
            for i in 0..group_rows(g) {
                let r = learn_row_at(g, i);
                for &act in r.act.iter().filter(|&&x| x != 0) {
                    assert_ne!(action_label(act), "?", "group {g} row {i}");
                }
            }
        }
    }
}
