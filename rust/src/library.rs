//! library.c: the home screen. Folders and notebooks as cards, paged (no scrolling on e-ink),
//! plus the sheets: new notebook (paper picker), settings, notebook actions, move, delete.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Modal {
    #[default]
    None = 0, // MOD_NONE
    New = 1,        // MOD_NEW
    Settings = 2,   // MOD_SETTINGS
    More = 3,       // MOD_MORE
    FolderMore = 4, // MOD_FOLDER_MORE
    Move = 5,       // MOD_MOVE
    Delete = 6,     // MOD_DELETE
    Kb = 7,         // MOD_KB
    Display = 8,    // MOD_DISPLAY
    Music = 9,      // MOD_MUSIC
    Language = 10,  // MOD_LANGUAGE
    Templates = 11, // MOD_TEMPLATES
    Koreader = 12,  // MOD_KOREADER
    Shortcuts = 13, // MOD_SHORTCUTS
    Buttons = 14,   // MOD_BUTTONS
    Pair = 15,      // MOD_PAIR
    Updates = 16,   // MOD_UPDATES
    Gestures = 17,  // MOD_GESTURES
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RecordPhase {
    #[default]
    Wait,
    Shape,
    Checking,
    Ready,
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
const H_MOVE_PREV: i32 = 69;
const H_MOVE_NEXT: i32 = 70;
const H_GESTURES: i32 = 58;
const H_GESTURE_GROUP: i32 = 59;
const H_GESTURE_ROW: i32 = 60;
const H_GESTURES_RESET: i32 = 61;
const H_GESTURES_DONE: i32 = 62;
const H_GESTURES_MASTER: i32 = 63;
const H_RECORD_USE: i32 = 64;
const H_RECORD_RETRY: i32 = 65;
const H_RECORD_CANCEL: i32 = 66;
const H_GESTURES_RESET_CONFIRM: i32 = 67;
const H_GESTURES_RESET_CANCEL: i32 = 68;

#[derive(Clone, Copy, Default)]
struct Item {
    is_folder: bool,
    /// index into books (notebooks)
    nb: usize,
    folder: FixedStr<STORE_NAME_LEN>,
    /// notebooks inside (folders)
    count: i32,
    has_children: bool,
}

/// The screen's state (library.c's, which app.h keeps opaque): library_init makes it, in a.lib.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct LibraryState {
    settings_return_editor: bool,
    settings_painting: bool,
    settings_editor_fit: bool,
    books: Vec<NotebookInfo>,
    folders: Vec<String>,
    items: Vec<Item>,
    /// "" = top level
    cur_folder: FixedStr<STORE_NAME_LEN>,
    folder_move_page: usize,
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
    // The Gestures sheet shares the action rows and binding descriptor with Buttons, while its
    // record card owns a one-finger stroke and the candidates being recorded.
    gestures_group: i32,
    gestures_learning: i32,
    gestures_file_readable: bool,
    gestures_refused: i32,
    gestures_reset_confirm: bool,
    record_action: i32,
    record_next_action: i32,
    record_started: u32,
    record_phase: RecordPhase,
    record_key: Option<GestureKey>,
    record_line: String,
    record_samples: [FsSample; FS_SAMPLES_MAX],
    record_sample_count: usize,
    record_checks_done: bool,
    record_stroke: FsStroke,
    record_previous_stroke: FsStroke,
    record_previous_lift: Option<FsLift>,
    record_previous_lift_ms: u32,
    record_touch_active: bool,
    record_attempted: bool,
    record_drawn: usize,
    record_trail_phase: i32,
    record_last_frame: u32,
    record_trail: Rect,
    record_same_action: i32,
    record_newer_confirm: bool,
    record_touch_origin: bool,
    record_two_strokes_pending: bool,
    // The card owns recognition independently of the editor and never sends pen ink.
    record_pb: PenButtons,
    record_pen: PbPen,
    record_pen_pointer: bool,
    record_pen_capture: bool,
    record_pen_redraw: bool,
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
    top_h()
        + margin()
        + if ui_theme() == UiTheme::Gothic {
            ui_px(24.0)
        } else {
            0
        }
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
    {
        for f in 0..s.folders.len() {
            if store_folder_parent(&s.folders[f]) != s.cur_folder.as_str() {
                continue;
            }
            let mut it = Item { is_folder: true, ..Default::default() };
            it.folder.set(&s.folders[f]);
            for i in 0..nbooks {
                it.count += store_folder_contains(&s.folders[f], s.books[i].folder.as_str()) as i32;
            }
            it.has_children = s.folders.iter().any(|child| child != &s.folders[f] && store_folder_contains(it.folder.as_str(), child));
            s.items.push(it);
        }
    }
    for i in 0..nbooks {
        if s.books[i].folder == s.cur_folder {
            s.items.push(Item {
                nb: i,
                ..Default::default()
            });
        }
    }
    let pages = (s.items.len() as i32 + pp - 1) / pp;
    s.page = clampi(s.page, 0, if pages > 0 { pages - 1 } else { 0 });
}

fn unique_name(
    s: &LibraryState,
    base: &FixedStr<STORE_NAME_LEN>,
    out: &mut FixedStr<STORE_NAME_LEN>,
) {
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
    bgimage_render(
        &mut full,
        s.books[nb].id.as_str(),
        s.books[nb].paper,
        Rect::new(0, 0, pw, ph),
    );
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
        if store_load_thumb(s.books[nb].id.as_str(), &mut s.thumbs[nb]) != 0
            && paper_is_image(s.books[nb].paper)
        {
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
        title = ui_name_label(store_folder_leaf(it.folder.as_str()), &mut label, LABEL_CAP);
        // TRANSLATORS: under a folder's card in the Library.
        csnprintf!(
            &mut line2,
            128,
            ng_("%d notebook", "%d notebooks", it.count as i64),
            it.count
        );
    } else {
        let nb = &s.books[it.nb];
        title = ui_name_label(nb.name.as_str(), &mut label, LABEL_CAP);
        let mut when = String::new();
        format_time_short(nb.modified, &mut when, 64);
        // TRANSLATORS: under a notebook's card in the Library: its pages and when it last changed (a
        // date and time, "Oct 10, 14:32"). Sideways the line has 316 px at 34 px; when it is longer,
        // the time is left out.
        csnprintf!(
            &mut line2,
            128,
            ng_("%d page, %s", "%d pages, %s", nb.npages() as i64),
            nb.npages(),
            &when
        );
    }
    let more = ui_px(110.0);
    let tr_ = Rect::new(
        r.x,
        r.y + th + ui_px(14.0),
        r.w - more - ui_px(10.0),
        ui_px(96.0),
    );
    // Gothic names use the pixel face's regular weight: its overprinted bold turns a name in
    // capitals into a solid black bar.
    let face = if ui_theme() == UiTheme::Gothic {
        FontFace::Regular
    } else {
        FontFace::Bold
    };
    ui_text_name(c, face, ui_text_size()*0.88, ui_small_size(), tr_, title, 2, 0);
    let lr = Rect::new(r.x, tr_.y + tr_.h, r.w - more - ui_px(10.0), ui_px(40.0));
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
            csnprintf!(
                &mut line2,
                128,
                ng_("%d page, %s", "%d pages, %s", nb.npages() as i64),
                nb.npages(),
                &day
            );
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
        Btn {
            icon: IconKind::Exit,
            label: tr("Exit"),
            id: H_EXIT,
            show: true,
        },
        // TRANSLATORS: the Library's top bar (see Exit).
        Btn {
            icon: IconKind::Gear,
            label: tr("Settings"),
            id: H_SETTINGS,
            show: true,
        },
        // TRANSLATORS: the Library's top bar (see Exit): to the Share screen (over Wi-Fi).
        Btn {
            icon: IconKind::Share,
            label: tr("Share"),
            id: H_SHARE,
            show: true,
        },
        // TRANSLATORS: the Library's top bar (see Exit): makes a new folder.
        Btn { icon: IconKind::FolderPlus, label: tr("Folder"), id: H_NEWFOLDER, show: true },
        // TRANSLATORS: the Library's top bar (see Exit): makes a new notebook.
        Btn {
            icon: IconKind::Plus,
            label: tr("New"),
            id: H_NEW,
            show: true,
        },
    ];
    // Started from KOReader's Grimoire plugin (kindle/app/grimoire.sh --hosted koreader), Exit goes
    // back to KOReader.
    let hosted = std::env::var_os("GRIMOIRE_HOST").is_some_and(|h| h == "koreader");
    if hosted {
        // The Library's top bar when Grimoire was opened from KOReader: in place of Exit, it opens
        // KOReader (a name, not translated), using the same button width as its neighbors.
        btns[0].label = "KOReader";
    }
    const NBTNS: usize = 5;
    // A word too long for its button (a translation) widens all five alike, as far as leaves the
    // app's name room in the bar (beyond that a word is cut, and the fit check hears of it).
    for (i, b) in btns.iter().enumerate() {
        let need = ui_button_need_w(
            bh,
            b.icon,
            b.label,
            if i == 4 {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Plain
            },
        );
        if b.show {
            bw = clampi(need, bw, ui_px(220.0));
        }
    }
    let mut rx = aw - m;
    for (i, b) in btns.iter().enumerate().take(NBTNS) {
        if !b.show {
            continue;
        }
        let w = bw;
        rx -= w;
        let r = Rect::new(rx, by, w, bh);
        ui_button(
            c,
            r,
            b.icon,
            b.label,
            if i == 4 {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Plain
            },
        );
        hits_add(&mut s.hits, r, b.id, 0);
        rx -= ui_px(20.0);
    }
    if !s.cur_folder.is_empty() {
        ui_icon(
            c,
            IconKind::Folder,
            Rect::new(x, by + ui_px(25.0), ui_px(100.0), ui_px(100.0)),
            0,
        );
        x += ui_px(115.0);
    } else if ui_theme() == UiTheme::Gothic {
        ui_emblem(
            c,
            (x + ui_px(55.0)) as f32,
            th as f32 / 2.0,
            ui_px(52.0) as f32,
            0,
            255,
        );
        x += ui_px(140.0);
    }
    // (A folder's name, cut short if it must be; else the app's.)
    let mut label = String::new();
    let title_px = ui_title_size()
        * if ui_theme() == UiTheme::Gothic {
            1.8
        } else {
            1.25
        };
    let title = if !s.cur_folder.is_empty() {
        ui_name_label(s.cur_folder.as_str(), &mut label, LABEL_CAP)
    } else {
        APP_NAME
    };
    let title_rect=Rect::new(x,0,rx-x-ui_px(20.0),th);
    if !s.cur_folder.is_empty() {
        ui_text_name(c,FontFace::Title,ui_title_size()*1.2,ui_small_size(),title_rect,title,2,0);
    } else {
        font_draw_left_name(c,FontFace::Title,title_px,title_rect,title,0);
    }
    ui_rule(c, 0, aw, th, ui_px(4.0), 0);
    ui_frieze(c, 0, aw, th + ui_px(9.0), ui_px(28.0), 0);

    let (cols, rows, cw, ch) = grid_geom(aw, ah);
    let pp = cols * rows;
    if empty_hint_shown(s) {
        let mut er = Rect::new(m, th + ui_px(300.0), aw - 2 * m, ui_px(100.0));
        // TRANSLATORS: the Library with nothing in it (1760 px at 64 px), and what to do (at 44 px).
        let empty = if !s.cur_folder.is_empty() {
            tr("This folder is empty")
        } else {
            tr("No notebooks yet")
        };
        font_draw_centered(c, FontFace::Bold, ui_title_size(), er, empty, 0);
        er.y += ui_px(110.0);
        // TRANSLATORS: New is the top bar's button (as translated there).
        font_draw_centered(
            c,
            FontFace::Regular,
            ui_text_size(),
            er,
            tr("Tap New to start one."),
            60,
        );
    }
    for i in 0..pp {
        let idx = s.page * pp + i;
        if idx >= s.items.len() as i32 {
            break;
        }
        let col = i % cols;
        let row = i / cols;
        draw_card(
            c,
            s,
            dpi,
            Rect::new(m + col * (cw + m), grid_top() + row * (ch + m), cw, ch),
            idx as usize,
        );
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
    ui_button(
        c,
        pr,
        IconKind::Prev,
        "",
        if s.page > 0 {
            ButtonStyle::Plain
        } else {
            ButtonStyle::Disabled
        },
    );
    ui_button(
        c,
        nr,
        IconKind::Next,
        "",
        if s.page < pages - 1 {
            ButtonStyle::Plain
        } else {
            ButtonStyle::Disabled
        },
    );
    if s.page > 0 {
        hits_add(&mut s.hits, pr, H_PREV, 0);
    }
    if s.page < pages - 1 {
        hits_add(&mut s.hits, nr, H_NEXT, 0);
    }
    let mut pl = String::new();
    // TRANSLATORS: between the Library's page buttons (1320 px at 44 px).
    csnprintf!(&mut pl, 96, tr("Page %d of %d"), s.page + 1, pages);
    font_draw_centered(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(pr.x + pr.w, pr.y, nr.x - pr.x - pr.w, pr.h),
        &pl,
        0,
    );
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
    let App {
        screen: c,
        lib,
        settings,
        ..
    } = a;
    let s = lib.as_deref_mut().unwrap();
    let card = sheet_rect(aw, ah, ui_px(2300.0));
    // TRANSLATORS: the title of the sheet that makes a notebook, and its heading over the papers.
    let in_ = ui_sheet(c, card, tr("New notebook"));
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(in_.x, in_.y, in_.w, ui_px(70.0)),
        tr("Paper"),
        0,
    );
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
        let tr_ = Rect::new(
            in_.x + (in_.w - gridw) / 2 + (i % g.cols) * (g.tw + gap),
            ty + (i / g.cols) * g.cell_h,
            g.tw,
            g.th,
        );
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
        ui_frame(
            c,
            rect_inflate(tr_, if sel { ui_px(10.0) } else { 0 }),
            ui_px(12.0),
            0,
            false,
            if sel { ui_px(9.0) } else { ui_px(3.0) },
        );
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
        hits_add(
            &mut s.hits,
            Rect::new(tr_.x, tr_.y, tr_.w, g.th + ui_px(90.0)),
            id,
            k.0 as i32,
        );
    }
    let mut y = ty + g.rows * g.cell_h + ui_px(20.0);
    let mut line = String::new();
    // TRANSLATORS: the new notebook's name, then its Rename button (268 px at 29 px, under its
    // icon, inside Gothic's carved line), then the folder it goes in, or the Library itself.
    csnprintf!(&mut line, 320, tr("Name: %s"), &s.new_name);
    let rn = Rect::new(in_.x + in_.w - ui_px(300.0), y, ui_px(300.0), ui_px(120.0));
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(in_.x, y, in_.w - ui_px(330.0), ui_px(120.0)),
        &line,
        0,
    );
    ui_button(c, rn, IconKind::Rename, tr("Rename"), ButtonStyle::Plain);
    hits_add(&mut s.hits, rn, H_NEW_RENAME, 0);
    y += ui_px(140.0);
    csnprintf!(
        &mut line,
        320,
        tr("In: %s"),
        if !s.cur_folder.is_empty() {
            s.cur_folder.as_str()
        } else {
            tr("Library (top level)")
        }
    );
    font_draw_left(
        c,
        FontFace::Regular,
        ui_text_size(),
        Rect::new(in_.x, y, in_.w, ui_px(80.0)),
        &line,
        60,
    );
    let bh = ui_px(140.0);
    let bw = ui_px(380.0);
    let cancel = Rect::new(in_.x, in_.y + in_.h - bh, bw, bh);
    let create = Rect::new(in_.x + in_.w - bw, in_.y + in_.h - bh, bw, bh);
    // TRANSLATORS: the sheets' buttons: 348 px at 44 px inside Gothic's carved line (342 for Create,
    // the main one), or at 34 px, or two lines of that.
    ui_button(c, cancel, IconKind::None, tr("Cancel"), ButtonStyle::Plain);
    ui_button(
        c,
        create,
        IconKind::None,
        tr("Create"),
        ButtonStyle::Primary,
    );
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
    MenuRow {
        icon,
        label: label.to_string(),
        id,
        arg,
        st,
    }
}

// ---- the controller's buttons (Settings > Controller > Buttons) ----

const LEARN_MS: u32 = 10000;
// A Buttons row held this long without moving loses its buttons (forget_row), as a link held in the
// editor opens (LINK_HOLD_MS).
const ROW_HOLD_MS: u32 = 600;

// One descriptor drives the Buttons and Gestures sheets. The shared registry supplies the seven
// action groups; these callbacks own only source-specific text and changes.
#[derive(Clone, Copy)]
struct BindKind {
    modal: Modal,
    title: &'static str,
    prompt: &'static str,
    sources: u8,
    row_text: fn(&App, i32, &mut String),
    learn: fn(&mut App, i32) -> bool,
    forget: fn(&mut App, LearnRow) -> bool,
    reset: fn(&mut App) -> bool,
}

const BUTTONS: BindKind = BindKind {
    modal: Modal::Buttons,
    title: n_("Controller buttons"),
    prompt: n_("Press a button for %s"),
    sources: SRC_KEY | SRC_PAD,
    row_text: buttons_row_text,
    learn: buttons_learn,
    forget: buttons_forget,
    reset: buttons_reset,
};

const GESTURES: BindKind = BindKind {
    modal: Modal::Gestures,
    title: n_("Gestures"),
    prompt: n_("Do the gesture for %s"),
    sources: SRC_TOUCH | SRC_SHAPE | SRC_PEN,
    row_text: gestures_row_text,
    learn: gestures_learn,
    forget: gestures_forget,
    reset: gestures_reset,
};

fn binding_row_count(kind: BindKind, group: usize) -> usize {
    let mut n = 0;
    for row in 0..group_row_count(group) {
        let Some(r) = learn_row_at(group, row) else {
            continue;
        };
        if action_sources(r.act[0]) & kind.sources != 0 {
            n += 1;
        }
    }
    n
}

fn binding_row_at(kind: BindKind, group: usize, wanted: usize) -> Option<LearnRow> {
    let mut visible = 0;
    for row in 0..group_row_count(group) {
        let r = learn_row_at(group, row)?;
        if action_sources(r.act[0]) & kind.sources == 0 {
            continue;
        }
        if visible == wanted {
            return Some(r);
        }
        visible += 1;
    }
    None
}

fn binding_row_index(kind: BindKind, group: usize, action: i32) -> Option<usize> {
    (0..binding_row_count(kind, group)).find(|&i| {
        binding_row_at(kind, group, i).is_some_and(|r| r.act[0] == action || r.act[1] == action)
    })
}

// The action the selected Buttons row waits for; paired rows learn both actions in turn.
fn learning_action(s: &LibraryState) -> i32 {
    binding_row_at(
        BUTTONS,
        s.buttons_group.max(0) as usize,
        s.learn_row.max(0) as usize,
    )
    .map_or(SHORTCUT_NONE, |r| r.act[s.learn_step.clamp(0, 1) as usize])
}

// "South, Up" for an action's buttons, "none" when it has none; "B, Up" while the first controller
// connected is a Switch-mode one, as its buttons are printed.
fn pads_text(a: &App, action: i32, out: &mut String, cap: usize) {
    let mut codes = [0i32; GROUP_ROWS_MAX];
    let n = shortcuts_pads_for(&a.shortcuts, ActionSrc::Pad, action, &mut codes);
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
    while i < n && i < GROUP_ROWS_MAX as i32 {
        let mut name = String::new();
        shortcut_pad_label(codes[i as usize], switch_layout, &mut name, 16);
        let len = out.len();
        let mut piece = String::new();
        csnprintf!(
            &mut piece,
            cap - len,
            "%s%s",
            if i != 0 { ", " } else { "" },
            &name
        );
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

fn gesture_path() -> String {
    gesturemap_path()
}

fn gesture_key_label(k: GestureKey, out: &mut String) {
    out.clear();
    // TRANSLATORS: names of the gestures shown in Settings > Gestures, in a row with an action.
    // Each name must fit in about 1000 px at 44 px bold when a shape thumbnail is also shown.
    match k {
        GestureKey::Touch {
            fingers: 1,
            kind: TouchKind::Up,
        } => out.push_str(tr("1 finger up")),
        GestureKey::Touch {
            fingers: 1,
            kind: TouchKind::Down,
        } => out.push_str(tr("1 finger down")),
        GestureKey::Touch {
            fingers: 1,
            kind: TouchKind::Left,
        } => out.push_str(tr("1 finger left")),
        GestureKey::Touch {
            fingers: 1,
            kind: TouchKind::Right,
        } => out.push_str(tr("1 finger right")),
        GestureKey::Touch {
            fingers: 2,
            kind: TouchKind::Up,
        } => out.push_str(tr("2 fingers up")),
        GestureKey::Touch {
            fingers: 2,
            kind: TouchKind::Down,
        } => out.push_str(tr("2 fingers down")),
        GestureKey::Touch {
            fingers: 2,
            kind: TouchKind::Left,
        } => out.push_str(tr("2 fingers left")),
        GestureKey::Touch {
            fingers: 2,
            kind: TouchKind::Right,
        } => out.push_str(tr("2 fingers right")),
        GestureKey::Touch {
            fingers: 2,
            kind: TouchKind::Tap,
        } => out.push_str(tr("2-finger tap")),
        GestureKey::Touch {
            fingers: 3,
            kind: TouchKind::Up,
        } => out.push_str(tr("3 fingers up")),
        GestureKey::Touch {
            fingers: 3,
            kind: TouchKind::Down,
        } => out.push_str(tr("3 fingers down")),
        GestureKey::Touch {
            fingers: 3,
            kind: TouchKind::Left,
        } => out.push_str(tr("3 fingers left")),
        GestureKey::Touch {
            fingers: 3,
            kind: TouchKind::Right,
        } => out.push_str(tr("3 fingers right")),
        GestureKey::Touch {
            fingers: 3,
            kind: TouchKind::Tap,
        } => out.push_str(tr("3-finger tap")),
        GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Tap } => out.push_str(tr("pen button click")),
        GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Double } => out.push_str(tr("pen button double-click")),
        GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Triple } => out.push_str(tr("pen button triple-click")),
        GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Hold } => out.push_str(tr("pen button hold")),
        GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Drag } => out.push_str(tr("pen button drag")),
        GestureKey::Pen { button: PenBtn::Button2, kind: PenKind::Tap } => out.push_str(tr("second pen button click")),
        GestureKey::Pen { button: PenBtn::Button2, kind: PenKind::Double } => out.push_str(tr("second pen button double-click")),
        GestureKey::Pen { button: PenBtn::Button2, kind: PenKind::Triple } => out.push_str(tr("second pen button triple-click")),
        GestureKey::Pen { button: PenBtn::Button2, kind: PenKind::Hold } => out.push_str(tr("second pen button hold")),
        GestureKey::Pen { button: PenBtn::Button2, kind: PenKind::Drag } => out.push_str(tr("second pen button drag")),
        GestureKey::Touch { .. } | GestureKey::Pen { button: PenBtn::Eraser, .. } => out.push_str(tr("no gesture")),
    }
}

// A source counts once even if several of its slots do the action. Unsupported
// three-finger slots must not hide a usable one-finger or pen binding on a two-slot panel.
fn gesture_touch_for_caps(a: &App, action: i32, touch_slots: i32) -> Option<GestureKey> {
    let mut touch = None;
    for fingers in [2, 3, 1] {
        if fingers == 3 && touch_slots < 3 { continue; }
        for kind in [TouchKind::Down, TouchKind::Up, TouchKind::Left, TouchKind::Right, TouchKind::Tap] {
            let key = GestureKey::Touch { fingers, kind };
            if gesturemap_action(&a.gestures, key) == action && action != SHORTCUT_NONE {
                touch = Some(key);
                break;
            }
        }
        if touch.is_some() { break; }
    }
    touch
}

fn gesture_row_binding_text(a: &App, actions: &[i32], touch_slots: i32, out: &mut String) {
    let (mut touch, mut pen) = (None, None);
    let (mut shape, mut newer) = (false, false);
    let mut sources = 0;
    for &action in actions.iter().filter(|&&action| action != SHORTCUT_NONE) {
        let t = gesture_touch_for_caps(a, action, touch_slots);
        let p = gesturemap_key_for(&a.gestures, action, ActionSrc::Pen);
        let s = gesturemap_shape_of(&a.gestures, action).is_some();
        let n = gesturemap_shape_newer(&a.gestures, action);
        sources += t.is_some() as i32 + (s || n) as i32 + p.is_some() as i32;
        touch = touch.or(t);
        pen = pen.or(p);
        shape |= s;
        newer |= n;
    }
    out.clear();
    if let Some(key) = touch {
        gesture_key_label(key, out);
    } else if shape {
        out.push_str(tr("finger shape"));
    } else if newer {
        out.push_str(tr("finger shape (newer version)"));
    } else if let Some(key) = pen {
        gesture_key_label(key, out);
    } else {
        out.push_str(tr("no gesture"));
    }
    if sources > 1 {
        // TRANSLATORS: extra binding sources on a Gestures row, after its first touch, shape or pen binding.
        let suffix = cformat!(tr("+%d"), sources - 1);
        out.push(' ');
        out.push_str(&suffix);
    }
}

fn gesture_binding_text_for_caps(a: &App, action: i32, touch_slots: i32, out: &mut String) {
    gesture_row_binding_text(a, &[action], touch_slots, out);
}

fn gesture_binding_text(a: &App, action: i32, out: &mut String) {
    gesture_binding_text_for_caps(a, action, input_caps().touch_slots, out);
}

fn buttons_row_text(a: &App, action: i32, out: &mut String) {
    pads_text(a, action, out, 256);
}

fn gestures_row_text(a: &App, action: i32, out: &mut String) {
    gesture_binding_text(a, action, out);
}

fn buttons_learn(a: &mut App, action: i32) -> bool {
    let Some(s) = a.lib.as_deref_mut() else {
        return false;
    };
    if s.modal != BUTTONS.modal || s.buttons_group < 0 {
        return false;
    }
    let Some(row) = binding_row_index(BUTTONS, s.buttons_group as usize, action) else {
        return false;
    };
    s.learn_row = row as i32;
    s.learn_step = 0;
    s.learn_at = now_ms();
    redraw(a, RefreshMode::Ui);
    true
}

fn buttons_forget(a: &mut App, row: LearnRow) -> bool {
    let cp = controller_path();
    if shortcuts_forget(
        &mut a.shortcuts,
        &cp,
        ActionSrc::Pad,
        row.act[0],
        row.act[1],
    ) != 0
    {
        app_toast(a, tr("Could not save controller.ini"), 4000);
        return false;
    }
    let mut msg = String::new();
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the Buttons sheet row now
    // has no controller button; %s is its name ("Undo", "Previous / next page").
    csnprintf!(
        &mut msg,
        200,
        tr("No buttons for %s now."),
        row.label
            .map(tr)
            .unwrap_or_else(|| action_label(row.act[0]))
    );
    app_toast(a, &msg, 2000);
    true
}

fn buttons_reset(a: &mut App) -> bool {
    let cp = controller_path();
    let _ = std::fs::remove_file(&cp);
    lib(a).learn_row = -1;
    reload_shortcuts(a);
    true
}

fn gestures_learn(a: &mut App, action: i32) -> bool {
    open_record_card(a, action)
}

fn save_gesture_map(a: &mut App, next: GestureMap) -> bool {
    if !lib(a).gestures_file_readable {
        app_toast(a, tr("Could not save the gestures."), 3000);
        return false;
    }
    if gesturemap_save(&next, &gesture_path()) < 0 {
        app_toast(a, tr("Could not save the gestures."), 3000);
        return false;
    }
    a.gestures = next;
    app_gestures_sync(a);
    true
}

fn gestures_forget(a: &mut App, row: LearnRow) -> bool {
    let mut next = a.gestures.clone();
    gesturemap_clear_action(&mut next, row.act[0]);
    if row.act[1] != 0 {
        gesturemap_clear_action(&mut next, row.act[1]);
    }
    if !save_gesture_map(a, next) {
        return false;
    }
    let mut msg = String::new();
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a finger held a Gestures
    // sheet row, whose touch, shape and pen bindings are clear; %s is its action or paired row name.
    csnprintf!(
        &mut msg,
        200,
        tr("No gestures for %s now."),
        row.label
            .map(tr)
            .unwrap_or_else(|| action_label(row.act[0]))
    );
    app_toast(a, &msg, 2000);
    true
}

fn gestures_reset(a: &mut App) -> bool {
    let n = gesturemap_shapes(&a.gestures).len();
    if n != 0 {
        lib(a).gestures_reset_confirm = true;
        redraw(a, RefreshMode::Ui);
        return true;
    }
    reset_gesture_map(a)
}

fn reset_gesture_map(a: &mut App) -> bool {
    let mut next = a.gestures.clone();
    if gesturemap_reset(&mut next, &gesture_path()) < 0 {
        app_toast(a, tr("Could not save the gestures."), 3000);
        return false;
    }
    a.gestures = next;
    app_gestures_sync(a);
    lib(a).gestures_file_readable = true;
    lib(a).gestures_refused = 0;
    lib(a).gestures_reset_confirm = false;
    redraw(a, RefreshMode::Ui);
    true
}

fn open_gestures(a: &mut App) {
    gesturemap_pen_button2(&mut a.gestures, input_caps().pen_button2);
    let n = gesturemap_load(&mut a.gestures, &gesture_path());
    app_gestures_sync(a);
    let s = lib(a);
    s.gestures_file_readable = n >= 0;
    s.gestures_refused = n.max(0);
    s.gestures_group = -1;
    s.gestures_learning = SHORTCUT_NONE;
    s.gestures_reset_confirm = false;
    s.record_action = SHORTCUT_NONE;
    s.record_next_action = SHORTCUT_NONE;
    if n > 0 {
        let mut msg = String::new();
        // TRANSLATORS: a toast (about 1700 px at 44 px bold or two lines): malformed or unknown
        // gestures.ini lines were skipped; %d is their count.
        csnprintf!(&mut msg, 200, ng_("%d line of gestures.ini could not be used.", "%d lines of gestures.ini could not be used.", n as i64), n);
        app_toast(a, &msg, 3500);
    } else if n < 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px bold or two lines): the current gestures.ini
        // is unreadable; Reset gestures is the recovery action.
        app_toast(a, tr("gestures.ini could not be read: Reset makes a new one."), 4000);
    }
    open_modal(a, Modal::Gestures);
}

fn open_record_card(a: &mut App, action: i32) -> bool {
    let now = now_ms();
    {
        let s = lib(a);
        s.gestures_learning = action;
        s.record_action = action;
        s.record_started = now;
        s.record_phase = RecordPhase::Wait;
        s.record_key = None;
        s.record_line.clear();
        s.record_samples = [FsSample::default(); FS_SAMPLES_MAX];
        s.record_sample_count = 0;
        s.record_checks_done = false;
        s.record_stroke = FsStroke::default();
        s.record_previous_stroke = FsStroke::default();
        s.record_previous_lift = None;
        s.record_previous_lift_ms = 0;
        s.record_touch_active = false;
        s.record_attempted = false;
        s.record_drawn = 0;
        s.record_trail_phase = 0;
        s.record_last_frame = 0;
        s.record_trail = RECT_EMPTY;
        s.record_same_action = 0;
        s.record_newer_confirm = false;
        s.record_touch_origin = false;
        s.record_two_strokes_pending = false;
        s.record_pb = PenButtons::default();
        s.record_pen = PbPen::default();
        s.record_pen_pointer = false;
        s.record_pen_capture = false;
        s.record_pen_redraw = false;
        s.pressed = None;
    }
    input_touch_context(TgView { gestures_on: true, record: true, ..TgView::default() });
    redraw(a, RefreshMode::Ui);
    true
}

fn close_record_card(a: &mut App) {
    input_touch_context(TgView { gestures_on: a.settings.gestures, ..TgView::default() });
    let s = lib(a);
    s.record_action = SHORTCUT_NONE;
    s.record_next_action = SHORTCUT_NONE;
    s.gestures_learning = SHORTCUT_NONE;
    s.record_phase = RecordPhase::Wait;
    s.record_key = None;
    s.record_touch_active = false;
    s.record_two_strokes_pending = false;
    s.record_pb = PenButtons::default();
    s.record_pen = PbPen::default();
    s.record_pen_pointer = false;
    s.record_pen_capture = false;
    s.record_pen_redraw = false;
    s.record_trail = RECT_EMPTY;
    s.record_line.clear();
    s.pressed = None;
    redraw(a, RefreshMode::Ui);
}

// Paired rows learn both directions, just as Buttons does. Each Use saves its own
// action before opening the other card; Cancel leaves an already saved binding intact.
fn finish_recording(a: &mut App) {
    let next = lib(a).record_next_action;
    if next != SHORTCUT_NONE {
        lib(a).record_next_action = SHORTCUT_NONE;
        open_record_card(a, next);
    } else {
        close_record_card(a);
    }
}

/// Whether the Library currently owns the raw gesture recording context.
pub fn library_record_active(a: &App) -> bool {
    a.lib.as_deref().is_some_and(|s| s.modal == Modal::Gestures && s.record_action != SHORTCUT_NONE)
}

fn redraw_record_card(a: &mut App) {
    // Ui is GL16 on the Kindle. Recognition may finish at contact, but its
    // display update waits for lift; no feedback may interrupt a pen drag.
    if lib(a).record_pen.touching {
        lib(a).record_pen_redraw = true;
        return;
    }
    lib(a).record_pen_redraw = false;
    let card = record_card_rect(a);
    // Samples/pattern and Use change below the fixed title and instructions. Restore any
    // earlier trail as well; a refusal with no trail uses only draw_record_line instead.
    let dynamic = Rect::new(card.x + ui_px(20.0), card.y + ui_px(180.0),
        card.w - ui_px(40.0), card.h - ui_px(200.0));
    let r = rect_union(dynamic, lib(a).record_trail);
    hits_clear(&mut lib(a).hits);
    draw_record_card(a);
    lib(a).record_trail = RECT_EMPTY;
    app_invalidate(a, r, RefreshMode::Ui);
}

fn record_refusal(a: &mut App, line: &str) {
    let s = lib(a);
    let restore_card = s.record_key.is_some() || !rect_is_empty(s.record_trail);
    s.record_attempted = true;
    s.record_key = None;
    s.record_phase = match s.record_sample_count {
        0 => RecordPhase::Wait,
        1 => RecordPhase::Shape,
        _ => RecordPhase::Ready,
    };
    s.record_line = line.to_string();
    if restore_card { redraw_record_card(a); } else { draw_record_line(a); }
}

// Only the screen receives this trail. Throttle display submissions to 25 Hz; lifting
// redraws the entire card so neither dots nor a discarded button press remain behind.
fn draw_record_trail(a: &mut App, now: u32) {
    let s = lib(a);
    if s.record_stroke.state != FsState::Drawing
        || (s.record_drawn != 0 && now.wrapping_sub(s.record_last_frame) < 40)
    {
        return;
    }
    let points = fs_points(&s.record_stroke);
    let n = points.len();
    if n < 2 || n == s.record_drawn {
        return;
    }
    let from = if n >= s.record_drawn { s.record_drawn.saturating_sub(1) } else { 0 };
    let mut xy = [(0.0f32, 0.0f32); FS_RAW_MAX];
    for (i, p) in points[from..].iter().enumerate() {
        xy[i] = (p.x as f32, p.y as f32);
    }
    let phase = s.record_trail_phase;
    let card = record_card_rect(a);
    let old_clip = a.screen.clip;
    canvas_set_clip(&mut a.screen, rect_intersect(old_clip, card));
    let phase = canvas_dot_poly(
        &mut a.screen, &xy[..n - from], ui_px(3.5).max(1),
        ui_px(20.0).max(1), ui_px(2.0).max(1), phase,
    );
    a.screen.clip = old_clip;
    let bounds = fs_box(&lib(a).record_stroke);
    let pad = ui_px(7.0).max(1);
    let r = rect_intersect(card, Rect::new(bounds.x - pad, bounds.y - pad, bounds.w + 2 * pad, bounds.h + 2 * pad));
    let s = lib(a);
    s.record_drawn = n;
    s.record_trail_phase = phase;
    s.record_last_frame = now;
    s.record_trail = rect_union(s.record_trail, r);
    app_invalidate(a, r, RefreshMode::Fast);
    app_flush(a);
}

fn record_pen_output(a: &mut App, out: PbOut) {
    let (button, kind) = match out {
        PbOut::Fire(button, kind) => (button, kind),
        PbOut::HoldBegin(button) => (button, PenKind::Hold),
        PbOut::DragBegin(button) => (button, PenKind::Drag),
        _ => return,
    };
    if button == PenBtn::Eraser || (button == PenBtn::Button2 && !input_caps().pen_button2) { return; }
    let s = lib(a);
    s.record_attempted = true;
    s.pressed = None;
    s.record_pen_pointer = false;
    if kind == PenKind::Drag { s.record_pen_capture = true; }
    fs_abort(&mut s.record_stroke);
    s.record_touch_active = false;
    if s.record_sample_count != 0 {
        record_refusal(a, tr("Not like the first one: draw it again."));
    } else {
        record_pattern(a, GestureKey::Pen { button, kind });
    }
}

// Return Some(consumed) for every pen event. Only a normal pen tap that starts
// on Use/Try again/Cancel reaches the Library's ordinary pointer handling.
fn record_pen_event(a: &mut App, ev: &InEvent) -> Option<bool> {
    if !matches!(ev.type_, InType::PenButton | InType::PenHover | InType::PenLeave
        | InType::PenDown | InType::PenMove | InType::PenUp) { return None; }
    let config = PbConfig { waits:[true,true,false], ..a.pb_config };
    let t = in_sample_ms(ev);
    if ev.type_ == InType::PenButton {
        let button = if ev.value == 2 { PenBtn::Button2 } else { PenBtn::Button };
        if button == PenBtn::Button2 && !input_caps().pen_button2 { return Some(true); }
        let down = if button == PenBtn::Button2 { ev.side_button2 } else { ev.side_button };
        let s = lib(a);
        s.record_pen.prox = true;
        s.record_attempted = true;
        let out = penbutton_event(&mut s.record_pb, &config, PbIn { t_ms: t, button, down }, s.record_pen);
        record_pen_output(a, out);
        return Some(true);
    }
    let pen = PbPen {
        prox: ev.type_ != InType::PenLeave,
        touching: matches!(ev.type_, InType::PenDown | InType::PenMove),
    };
    lib(a).record_pen = pen;
    // Two buttons may end together on lift/leave. Drain all outputs, including ends.
    for _ in 0..3 {
        let out = penbutton_pen(&mut lib(a).record_pb, &config, pen, t);
        if out == PbOut::None { break; }
        record_pen_output(a, out);
    }
    if ev.type_ == InType::PenDown {
        let pointer = !ev.side_button && !ev.side_button2 && !lib(a).record_pen_capture
            && hits_find(&lib(a).hits, ev.x, ev.y)
                .is_some_and(|h| matches!(h.id, H_RECORD_USE | H_RECORD_RETRY | H_RECORD_CANCEL));
        lib(a).record_pen_pointer = pointer;
    }
    let pointer = lib(a).record_pen_pointer;
    if !pen.touching {
        let s = lib(a);
        s.record_pen_pointer = false;
        s.record_pen_capture = false;
        if ev.type_ == InType::PenLeave { s.record_pen_redraw |= s.pressed.take().is_some(); }
        if s.record_pen_redraw { redraw_record_card(a); }
    }
    Some(!pointer || matches!(ev.type_, InType::PenHover | InType::PenLeave))
}

fn record_event(a: &mut App, ev: &InEvent) -> bool {
    if let Some(consumed) = record_pen_event(a, ev) { return consumed; }
    if ev.type_ == InType::Gesture {
        let s = lib(a);
        s.record_attempted = true;
        s.record_touch_active = false;
        s.pressed = None;
        fs_abort(&mut s.record_stroke);
        if ev.value < 0 {
            record_refusal(a, record_reject_line(ev.value));
        } else if lib(a).record_sample_count == 0 {
            if let Some(key) = gesturemap_key_of_id(ev.value) {
                record_pattern(a, key);
            }
        } else {
            // TRANSLATORS: the record card already has a shape sample; repeat that shape.
            record_refusal(a, tr("Not like the first one: draw it again."));
        }
        return true;
    }
    if !matches!(ev.type_, InType::TouchDown | InType::TouchMove | InType::TouchUp) {
        return false;
    }
    let t = in_sample_ms(ev);
    if ev.type_ == InType::TouchDown {
        if !rect_contains(record_card_rect(a), ev.x as i32, ev.y as i32) {
            return true;
        }
        let button = hits_find(&lib(a).hits, ev.x, ev.y)
            .is_some_and(|h| matches!(h.id, H_RECORD_USE | H_RECORD_RETRY | H_RECORD_CANCEL));
        let px100 = (a.dpi / 25.4 * 100.0).round() as i32;
        let s = lib(a);
        s.record_two_strokes_pending = !button && s.record_previous_lift.is_some_and(|lift|
            fs_two_strokes(&s.record_previous_stroke, lift, s.record_previous_lift_ms, t));
        fs_begin(&mut s.record_stroke, ev.x.round_ties_even() as i32, ev.y.round_ties_even() as i32, t, px100);
        if ev.value & TOUCH_F_BROAD != 0 {
            fs_kill(&mut s.record_stroke, FsReason::Broad);
        }
        s.record_touch_active = true;
        s.record_touch_origin = !button;
        s.record_attempted |= !button;
        s.record_drawn = 0;
        s.record_trail_phase = 0;
        s.record_last_frame = 0;
        if s.record_two_strokes_pending {
            record_refusal(a, shape_refusal_line(FsReason::TwoStrokes));
        }
        return !button;
    }
    if !lib(a).record_touch_active {
        return true;
    }
    if touch_cancelled(ev) {
        let s = lib(a);
        let restore_card = s.pressed.is_some() || !rect_is_empty(s.record_trail);
        fs_abort(&mut s.record_stroke);
        s.record_touch_active = false;
        s.pressed = None;
        if restore_card { redraw_record_card(a); }
        return true;
    }
    let state = fs_add(&mut lib(a).record_stroke, ev.x.round_ties_even() as i32,
        ev.y.round_ties_even() as i32, t, ev.value & TOUCH_F_BROAD != 0);
    if state == FsState::Drawing && !lib(a).record_touch_origin {
        let s = lib(a);
        s.record_touch_origin = true;
        s.record_attempted = true;
        if let Some(h) = s.pressed.take() {
            canvas_invert(&mut a.screen, h.r);
            app_invalidate(a, h.r, RefreshMode::Ui);
        }
    }
    if ev.type_ == InType::TouchMove {
        draw_record_trail(a, ev.t_ms);
        return lib(a).record_touch_origin;
    }
    let mut sample = FsSample::default();
    let s = lib(a);
    let lift = fs_finish(&mut s.record_stroke, t, FsMode::Record, &mut sample);
    s.record_touch_active = false;
    if !s.record_touch_origin {
        return false; // A button tap keeps its normal pressed/release handling.
    }
    s.pressed = None;
    s.record_previous_stroke = std::mem::take(&mut s.record_stroke);
    s.record_previous_lift = Some(lift);
    s.record_previous_lift_ms = t;
    if s.record_two_strokes_pending {
        record_refusal(a, shape_refusal_line(FsReason::TwoStrokes));
    } else {
        match lift {
            FsLift::Shape if lib(a).record_key.is_none() && lib(a).record_sample_count < FS_SAMPLES_MAX => {
                redraw_record_card(a);
                record_shape_sample(a, sample);
            }
            FsLift::Swipe(dir) if lib(a).record_sample_count == 0 => record_pattern(a, GestureKey::Touch {
                fingers: 1,
                kind: match dir { FsSwipe::Up => TouchKind::Up, FsSwipe::Down => TouchKind::Down,
                    FsSwipe::Left => TouchKind::Left, FsSwipe::Right => TouchKind::Right },
            }),
            FsLift::Nothing => {
                // TRANSLATORS: a single-finger tap or hold on the record card cannot be assigned.
                record_refusal(a, tr("One finger is kept for taps and links."));
            }
            FsLift::Refused(reason) => record_refusal(a, shape_refusal_line(reason)),
            _ => redraw_record_card(a),
        }
    }
    true
}

fn record_touch_name(key: GestureKey, out: &mut String) {
    gesture_key_label(key, out);
}

fn record_reject_line(code: i32) -> &'static str {
    // TRANSLATORS: refusal lines on the gesture recording card, about 1320 px at 44 px or two lines at 34 px.
    match code {
        -1 => tr("The pen is near: move it away."),
        -2 => tr("Put the fingers down together."),
        -3 => tr("Swipe in a straight line."),
        -4 => tr("Swipe farther."),
        -5 => tr("Move both fingers the same way."),
        -6 => tr("Your hand moved: try again."),
        -7 => tr("Two fingers spreading is kept for zoom."),
        -9 => tr("Keep the fingers 1 to 8 cm apart."),
        -10 => tr("Too slow: do it in one go."),
        -11 => tr("This Kindle feels two fingers at most."),
        _ => tr("Your hand moved: try again."),
    }
}

fn shape_refusal_line(reason: FsReason) -> &'static str {
    // TRANSLATORS: shape recording refusal lines, about 1320 px at 44 px or two lines at 34 px.
    match reason {
        FsReason::TooSmall => tr("Draw it bigger: at least 2 cm across."),
        FsReason::TooSlow => tr("Too slow: draw it in one go."),
        FsReason::LateStart => tr("Start moving as soon as you touch."),
        FsReason::Paused => tr("You held still, so it stopped."),
        FsReason::Straight => tr("Too straight: give it two turns."),
        FsReason::Tangled => tr("Too many turns: draw a simpler shape."),
        FsReason::Broad => tr("Use a fingertip, not the hand."),
        FsReason::TwoStrokes => tr("One stroke, without lifting."),
        FsReason::Full => tr("Grimoire keeps up to 32 shapes."),
        _ => tr("Draw it bigger: at least 2 cm across."),
    }
}

fn record_slot_refusal(action: i32, key: GestureKey) -> Option<&'static str> {
    if action == SHORTCUT_HOLD_ERASER && !matches!(key,
        GestureKey::Pen { kind: PenKind::Hold | PenKind::Drag, .. }) {
        // TRANSLATORS: record card refusal, at most two lines at 34 px in 1292 px.
        return Some(tr("Erase while held needs the button held: hold it or drag with it."));
    }
    if matches!(action, SHORTCUT_PEN_PAN | SHORTCUT_PEN_ZOOM) && !matches!(key,
        GestureKey::Pen { kind: PenKind::Drag, .. }) {
        // TRANSLATORS: record card refusal for Pan/Zoom with the pen; same two-line budget.
        return Some(tr("Hold the pen button and drag with it."));
    }
    let source = match key { GestureKey::Touch { .. } => SRC_TOUCH, GestureKey::Pen { .. } => SRC_PEN };
    if action_sources(action) & source == 0 {
        return Some(tr("Use the pen button for this action."));
    }
    None
}

fn record_pattern(a: &mut App, key: GestureKey) {
    let action = lib(a).record_action;
    if let Some(line) = record_slot_refusal(action, key) {
        record_refusal(a, line);
        return;
    }
    let losing = gesturemap_action(&a.gestures, key);
    let name = {
        let mut out = String::new();
        record_touch_name(key, &mut out);
        out
    };
    let s = lib(a);
    s.record_key = Some(key);
    s.record_phase = RecordPhase::Ready;
    s.record_same_action = losing;
    s.record_checks_done = true;
    s.record_newer_confirm = false;
    if losing != 0 && losing != s.record_action {
        let mut line = String::new();
        // TRANSLATORS: the record card's status line (1320 px at 44 px or two lines at 34 px);
        // %s is the action that currently owns this gesture.
        csnprintf!(&mut line, 256, tr("Now for %s. Use moves it here."), action_label(losing));
        s.record_line = line;
    } else {
        s.record_line = name;
    }
    redraw_record_card(a);
}

fn check_shape_candidate(a: &mut App) -> bool {
    let (action, n, samples) = {
        let s = lib(a);
        (s.record_action, s.record_sample_count, s.record_samples)
    };
    if n < 2 {
        return false;
    }
    let mut candidate = FsGesture::default();
    fs_prepare(&mut candidate, &samples[..n]);
    let mut others = [FsGesture::default(); FS_SHAPES_MAX];
    let mut other_actions = [0i32; FS_SHAPES_MAX];
    let mut no = 0;
    for &(other_action, gesture) in gesturemap_shapes(&a.gestures) {
        if other_action != action {
            others[no] = gesture;
            other_actions[no] = other_action;
            no += 1;
        }
    }
    match fs_record_separation(&candidate, &others[..no]) {
        FsSeparation::SameAs(i) => {
            let loser = other_actions[i];
            let mut line = String::new();
            // TRANSLATORS: the record card's status line; %s is the action that already owns this shape.
            csnprintf!(&mut line, 256, tr("That is the shape for %s. Use moves it here."), action_label(loser));
            let s = lib(a);
            s.record_same_action = loser;
            s.record_checks_done = true;
            s.record_phase = RecordPhase::Ready;
            s.record_line = line;
            redraw_record_card(a);
            return true;
        }
        FsSeparation::TooClose(i) => {
            let mut line = String::new();
            // TRANSLATORS: the record card's status line; %s is the action whose shape is too close.
            csnprintf!(&mut line, 256, tr("Too close to the shape for %s."), action_label(other_actions[i]));
            let s = lib(a);
            s.record_sample_count = 0;
            s.record_samples = [FsSample::default(); FS_SAMPLES_MAX];
            s.record_checks_done = false;
            s.record_phase = RecordPhase::Wait;
            s.record_line = line;
            redraw_record_card(a);
            return false;
        }
        FsSeparation::Clear => {}
    }
    {
        let s = lib(a);
        s.record_phase = RecordPhase::Checking;
        s.record_line = tr("Checking that a slip cannot draw it...").to_string();
    }
    draw_record_line(a);
    if fs_record_bank(&candidate, &mut work_pump).is_err() {
        let s = lib(a);
        s.record_sample_count = 0;
        s.record_samples = [FsSample::default(); FS_SAMPLES_MAX];
        s.record_checks_done = false;
        s.record_phase = RecordPhase::Wait;
        // TRANSLATORS: two lines on the record card when a slip or palm could match this shape.
        s.record_line = cformat!("%s\n%s", tr("Too easy to make by accident."), tr("Give it two turns or a loop."));
        redraw_record_card(a);
        return false;
    }
    let s = lib(a);
    s.record_phase = RecordPhase::Ready;
    s.record_checks_done = true;
    s.record_line.clear();
    redraw_record_card(a);
    true
}

fn record_shape_sample(a: &mut App, sample: FsSample) {
    let action = lib(a).record_action;
    if action_sources(action) & SRC_SHAPE == 0 {
        let key = GestureKey::Touch { fingers: 1, kind: TouchKind::Up };
        record_refusal(a, record_slot_refusal(action, key).unwrap_or(tr("Use the pen button for this action.")));
        return;
    }
    let count = lib(a).record_sample_count;
    if count == 0 {
        let current = gesturemap_shapes(&a.gestures).len();
        let action = lib(a).record_action;
        let has_own = gesturemap_shape_of(&a.gestures, action).is_some();
        if fs_record_room(current.saturating_sub(has_own as usize)).is_err() {
            let s = lib(a);
            s.record_phase = RecordPhase::Wait;
            s.record_line = shape_refusal_line(FsReason::Full).to_string();
            redraw_record_card(a);
            return;
        }
        {
            let s = lib(a);
            s.record_phase = RecordPhase::Checking;
            s.record_line = tr("Checking that a slip cannot draw it...").to_string();
        }
        draw_record_line(a);
        if fs_record_first(&sample, &mut work_pump).is_err() {
            let s = lib(a);
            s.record_phase = RecordPhase::Wait;
            // TRANSLATORS: two lines on the record card when a slip or palm could match this shape.
            s.record_line = cformat!("%s\n%s", tr("Too easy to make by accident."), tr("Give it two turns or a loop."));
            redraw_record_card(a);
            return;
        }
        let s = lib(a);
        s.record_samples[0] = sample;
        s.record_sample_count = 1;
        s.record_checks_done = false;
        s.record_phase = RecordPhase::Shape;
        s.record_line = tr("Good. Draw it again (2 of 3).").to_string();
        redraw_record_card(a);
        return;
    }
    let earlier = lib(a).record_samples;
    if fs_record_consistent(&sample, &earlier[..count]).is_err() {
        let s = lib(a);
        s.record_phase = RecordPhase::Shape;
        s.record_line = tr("Not like the first one: draw it again.").to_string();
        redraw_record_card(a);
        return;
    }
    {
        let s = lib(a);
        s.record_samples[count] = sample;
        s.record_sample_count = count + 1;
        s.record_checks_done = false;
        s.record_same_action = 0;
        s.record_newer_confirm = false;
        s.record_phase = RecordPhase::Ready;
        s.record_line = if count == 1 { tr("Once more makes it surer, or Use.").to_string() } else { String::new() };
    }
    if count + 1 == FS_SAMPLES_MAX {
        let _ = check_shape_candidate(a);
    } else {
        redraw_record_card(a);
    }
}

fn use_recording(a: &mut App) {
    if lib(a).record_phase != RecordPhase::Ready {
        return;
    }
    if lib(a).record_key.is_none() && !lib(a).record_checks_done {
        if !check_shape_candidate(a) {
            return;
        }
        // After two samples Use runs the checks. Leave a newly discovered reassignment
        // visible until the next Use, just as when the third sample ran those checks.
        if lib(a).record_same_action != 0 {
            return;
        }
    }
    let action = lib(a).record_action;
    if let Some(key) = lib(a).record_key {
        let mut next = a.gestures.clone();
        let loser = gesturemap_set(&mut next, key, action);
        if loser < 0 || !save_gesture_map(a, next) {
            app_toast(a, tr("Could not save the gestures."), 3000);
            return;
        }
        if loser != 0 && loser != action {
            let mut msg = String::new();
            // TRANSLATORS: a toast when a touch or pen gesture is reassigned; %s is the action that loses it.
            csnprintf!(&mut msg, 200, tr("That gesture is no longer for %s."), action_label(loser));
            app_toast(a, &msg, 2500);
        }
        finish_recording(a);
        return;
    }
    if gesturemap_shape_newer(&a.gestures, action) && !lib(a).record_newer_confirm {
        let s = lib(a);
        s.record_newer_confirm = true;
        s.record_line = tr("This replaces a shape from a newer Grimoire.").to_string();
        redraw_record_card(a);
        return;
    }
    let samples = lib(a).record_samples;
    let count = lib(a).record_sample_count;
    let mut next = a.gestures.clone();
    let loser = gesturemap_shape_set(&mut next, action, &samples[..count]);
    if loser < 0 || !save_gesture_map(a, next) {
        app_toast(a, tr("Could not save the gestures."), 3000);
        return;
    }
    if loser != 0 && loser != action {
        let mut msg = String::new();
        // TRANSLATORS: a toast when a recorded finger shape is moved; %s is the action that loses it.
        csnprintf!(&mut msg, 200, tr("That shape is no longer for %s."), action_label(loser));
        app_toast(a, &msg, 2500);
    }
    finish_recording(a);
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
    let Some(s) = a.lib.as_deref() else {
        return false;
    };
    if s.modal != Modal::Buttons || s.buttons_group < 0 || s.learn_row < 0 {
        return false;
    }
    if !gamepad {
        return true; // (a keyboard's key is shortcuts.ini's: nothing to learn here)
    }
    let cp = controller_path();
    let action = learning_action(s);
    let before = shortcuts_lookup(&a.shortcuts, ActionSrc::Pad, code, 0);
    if shortcuts_learn(&mut a.shortcuts, &cp, ActionSrc::Pad, code, action) != 0 {
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
        csnprintf!(
            &mut msg,
            200,
            tr("That button is no longer for %s."),
            action_label(before)
        );
        app_toast(a, &msg, 2500);
    }
    let s = lib(a);
    if s.learn_step == 0 && binding_row_at(BUTTONS, s.buttons_group as usize, s.learn_row as usize).unwrap().act[1] != 0 {
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
    let Some(r) = binding_row_at(BUTTONS, lib(a).buttons_group as usize, row as usize) else { return };
    let mut msg = String::new();
    let cp = controller_path();
    lib(a).learn_row = -1;
    if shortcuts_forget(&mut a.shortcuts, &cp, ActionSrc::Pad, r.act[0], r.act[1]) != 0 {
        app_toast(a, tr("Could not save controller.ini"), 4000);
    } else {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a finger held on a
        // Buttons sheet row, whose actions no controller button does now; %s is the row's name as the
        // sheet shows it ("Undo", "Previous / next page").
        csnprintf!(
            &mut msg,
            200,
            tr("No buttons for %s now."),
            match r.label {
                Some(l) => tr(l),
                None => action_label(r.act[0]),
            }
        );
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
        csnprintf!(
            out,
            cap,
            "%s",
            if a.settings.shortcuts_enabled {
                tr("No controller or keyboard connected")
            } else {
                ""
            }
        );
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
            csnprintf!(
                &mut path,
                160,
                "/pair?addr=%s&protocol=%s",
                &s.pair_addr,
                if classic { "classic" } else { "ble" }
            );
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
    let Some(s) = a.lib.as_deref_mut() else {
        return;
    };
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
                app_toast(
                    a,
                    if ok && !r.pairing {
                        tr("Paired. The controller connects in a moment.")
                    } else {
                        tr("Could not pair that controller")
                    },
                    3000,
                );
                bt_ask(lib(a), ASK_STATUS, 0);
            }
        }
        ASK_REMOVE => {
            // TRANSLATORS: a toast: the controller is no longer paired with the Kindle.
            app_toast(
                a,
                if ok {
                    tr("Forgotten.")
                } else {
                    tr("Could not forget that controller")
                },
                2500,
            );
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
    s.upd = if e == UpdateEvent::Done {
        UPD_DONE
    } else {
        UPD_FAILED
    };
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
        ui_frame(
            c,
            r,
            ui_px(16.0),
            if row.st == ButtonStyle::Selected {
                0
            } else {
                255
            },
            true,
            0,
        );
        ui_frame(
            c,
            r,
            ui_px(16.0),
            if row.st == ButtonStyle::Disabled {
                170
            } else {
                0
            },
            false,
            if row.st == ButtonStyle::Primary {
                ui_px(6.0)
            } else {
                ui_px(3.0)
            },
        );
        let fg: u8 = if row.st == ButtonStyle::Selected {
            255
        } else if row.st == ButtonStyle::Disabled {
            170
        } else {
            0
        };
        if row.icon != IconKind::None {
            ui_icon(
                c,
                row.icon,
                Rect::new(
                    r.x + ui_px(20.0),
                    r.y + ui_px(15.0),
                    rh - ui_px(30.0),
                    rh - ui_px(30.0),
                ),
                fg,
            );
        }
        let text_rect = Rect::new(r.x + rh + ui_px(10.0), r.y, r.w - rh - ui_px(30.0), r.h);
        if row.id == H_NONE && row.st == ButtonStyle::Disabled {
            ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), text_rect, &row.label, 2, false, fg);
        } else {
            font_draw_left(c, FontFace::Bold, ui_text_size(), text_rect, &row.label, fg);
        }
        if row.st != ButtonStyle::Disabled {
            hits_add(&mut s.hits, r, row.id, row.arg);
        }
    }
}

fn bind_group(kind: BindKind, s: &LibraryState) -> i32 {
    if kind.modal == BUTTONS.modal {
        s.buttons_group
    } else {
        s.gestures_group
    }
}

fn bind_group_set(kind: BindKind, s: &mut LibraryState, group: i32) {
    if kind.modal == BUTTONS.modal {
        s.buttons_group = group;
    } else {
        s.gestures_group = group;
    }
}

fn bind_group_hit(kind: BindKind) -> i32 {
    if kind.modal == BUTTONS.modal {
        H_BUTTON_GROUP
    } else {
        H_GESTURE_GROUP
    }
}

fn bind_row_hit(kind: BindKind) -> i32 {
    if kind.modal == BUTTONS.modal {
        H_BUTTON_ROW
    } else {
        H_GESTURE_ROW
    }
}

fn bind_done_hit(kind: BindKind) -> i32 {
    if kind.modal == BUTTONS.modal {
        H_BUTTONS_DONE
    } else {
        H_GESTURES_DONE
    }
}

fn bind_reset_hit(kind: BindKind) -> i32 {
    if kind.modal == BUTTONS.modal {
        H_BUTTONS_RESET
    } else {
        H_GESTURES_RESET
    }
}

fn bind_row_text(a: &App, kind: BindKind, row: LearnRow, out: &mut String) {
    let label = row.label.map(tr).unwrap_or_else(|| action_label(row.act[0]));
    let mut first = String::new();
    if kind.modal == GESTURES.modal {
        gesture_row_binding_text(a, &row.act, input_caps().touch_slots, &mut first);
        // TRANSLATORS: Gestures row: action and first binding plus +N, about 1300 px with a thumbnail, two lines at 34 px.
        csnprintf!(out, 256, c_("gesture row", "%s: %s"), label, &first);
        return;
    }
    (kind.row_text)(a, row.act[0], &mut first);
    if row.act[1] != 0 {
        let mut second = String::new();
        (kind.row_text)(a, row.act[1], &mut second);
        csnprintf!(out, 256, "%s: %s / %s", label, &first, &second);
    } else {
        csnprintf!(out, 256, "%s: %s", label, &first);
    }
}

fn bind_menu_rows(a: &App, kind: BindKind) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = Vec::with_capacity(N_GROUPS + 4);
    if kind.modal == GESTURES.modal {
        // TRANSLATORS: Settings > Gestures' master switch row, about 1200 px at 44 px bold.
        rows.push(menu_row(
            IconKind::Gestures,
            tr(if a.settings.gestures {
                "Gestures: on (tap to change)"
            } else {
                "Gestures: off (tap to change)"
            }),
            H_GESTURES_MASTER,
            0,
            ButtonStyle::Plain,
        ));
        // TRANSLATORS: a short example below the Gestures sheet's master switch, about 1300 px
        // at 44 px bold or two lines at 34 px.
        rows.push(menu_row(
            IconKind::None,
            tr("Ideas: a two-finger tap for Undo, a finger shape for Pages."),
            H_NONE,
            0,
            ButtonStyle::Disabled,
        ));
    }
    for (i, grp) in ACTION_GROUPS.iter().enumerate() {
        if binding_row_count(kind, i) == 0 {
            continue;
        }
        rows.push(menu_row(
            grp.icon,
            tr(grp.label),
            bind_group_hit(kind),
            i as i32,
            ButtonStyle::Plain,
        ));
    }
    let reset_text = if kind.modal == BUTTONS.modal {
        tr("Reset buttons")
    } else {
        tr("Reset gestures")
    };
    let reset_enabled = if kind.modal == BUTTONS.modal {
        std::fs::metadata(controller_path()).is_ok()
    } else {
        true
    };
    rows.push(menu_row(
        IconKind::Trash,
        reset_text,
        bind_reset_hit(kind),
        0,
        if reset_enabled {
            ButtonStyle::Plain
        } else {
            ButtonStyle::Disabled
        },
    ));
    rows.push(menu_row(
        IconKind::Back,
        tr("Done"),
        bind_done_hit(kind),
        0,
        ButtonStyle::Plain,
    ));
    rows
}

fn draw_bind_sheet(a: &mut App, kind: BindKind) {
    let group = bind_group(kind, lib(a));
    if group < 0 {
        let rows = bind_menu_rows(a, kind);
        draw_menu(a, tr(kind.title), &rows);
    } else {
        draw_bind_group(a, kind, group as usize);
    }
}

fn draw_bind_group(a: &mut App, kind: BindKind, group: usize) {
    let n = binding_row_count(kind, group) as i32;
    let mut rh = ui_px(140.0);
    let mut gap = ui_px(20.0);
    let room = a.h - 2 * ui_px(40.0) - ui_px(120.0) - ui_px(72.0);
    if (n + 1) * (rh + gap) > room {
        rh = clampi(room / (n + 1) - gap, ui_px(118.0), rh);
    }
    if (n + 1) * (rh + gap) > room {
        gap = clampi(room / (n + 1) - rh, ui_px(10.0), gap);
    }
    let card = sheet_rect(a.w, a.h, ui_px(120.0) + ui_px(72.0) + (n + 1) * (rh + gap));
    let title = tr(ACTION_GROUPS[group].title);
    let in_ = ui_sheet(&mut a.screen, card, title);
    for i in 0..n as usize {
        let r = binding_row_at(kind, group, i).unwrap();
        let learning = kind.modal == BUTTONS.modal
            && lib(a).buttons_group == group as i32
            && lib(a).learn_row == i as i32;
        let mut text = String::new();
        if learning {
            let step = lib(a).learn_step.clamp(0, 1) as usize;
            csnprintf!(&mut text, 256, tr(kind.prompt), action_label(r.act[step]));
        } else {
            bind_row_text(a, kind, r, &mut text);
        }
        let thumb = if kind.modal == GESTURES.modal {
            gesturemap_shape_of(&a.gestures, r.act[0])
                .or_else(|| gesturemap_shape_of(&a.gestures, r.act[1]))
                .copied()
        } else {
            None
        };
        let row_rect = Rect::new(in_.x, in_.y + i as i32 * (rh + gap), in_.w, rh);
        let App { screen: c, lib, .. } = a;
        draw_bind_row(
            c,
            &mut lib.as_deref_mut().unwrap().hits,
            kind,
            r,
            i,
            row_rect,
            rh,
            learning,
            &text,
            thumb,
        );
    }
    let done = Rect::new(in_.x, in_.y + n * (rh + gap), in_.w, rh);
    let App { screen: c, lib, .. } = a;
    ui_button(c, done, IconKind::Back, tr("Done"), ButtonStyle::Plain);
    hits_add(
        &mut lib.as_deref_mut().unwrap().hits,
        done,
        bind_done_hit(kind),
        0,
    );
}

fn draw_bind_row(
    c: &mut Canvas,
    hits: &mut HitList,
    kind: BindKind,
    row: LearnRow,
    index: usize,
    r: Rect,
    rh: i32,
    learning: bool,
    text: &str,
    thumb: Option<FsGesture>,
) {
    let selected = learning;
    ui_frame(c, r, ui_px(16.0), if selected { 0 } else { 255 }, true, 0);
    ui_frame(
        c,
        r,
        ui_px(16.0),
        0,
        false,
        ui_px(if selected { 6.0 } else { 3.0 }),
    );
    let fg = if selected { 255 } else { 0 };
    if row.icon != IconKind::None {
        ui_icon(
            c,
            row.icon,
            Rect::new(
                r.x + ui_px(20.0),
                r.y + ui_px(15.0),
                rh - ui_px(30.0),
                rh - ui_px(30.0),
            ),
            fg,
        );
    }
    let thumb_w = if thumb.is_some() { ui_px(120.0) } else { 0 };
    let text_x = r.x + rh + ui_px(10.0);
    let text_w = r.w - rh - ui_px(30.0) - thumb_w;
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size(),
        ui_small_size(),
        Rect::new(text_x, r.y, text_w, r.h),
        text,
        2,
        false,
        fg,
    );
    if let Some(g) = thumb.as_ref() {
        let thumb_rect = Rect::new(
            r.x + r.w - ui_px(108.0),
            r.y + (r.h - ui_px(96.0)) / 2,
            ui_px(96.0),
            ui_px(96.0),
        );
        draw_shape_thumb(c, thumb_rect, g);
    }
    hits_add(hits, r, bind_row_hit(kind), index as i32);
}

fn draw_shape_thumb(c: &mut Canvas, r: Rect, g: &FsGesture) {
    canvas_fill(c, r, 255);
    canvas_rect_outline(c, r, ui_px(2.0), 0);
    if g.n == 0 {
        return;
    }
    let old_clip = c.clip;
    canvas_set_clip(c, rect_intersect(old_clip, r));
    let pad = ui_px(10.0);
    let side_x = (r.w - 2 * pad).max(1);
    let side_y = (r.h - 2 * pad).max(1);
    let sample = &g.samples[fs_medoid(g).min(g.n - 1)];
    for i in 1..FS_N {
        let x0 = r.x + pad + sample.xy[(i - 1) * 2] as i32 * side_x / 255;
        let y0 = r.y + pad + sample.xy[(i - 1) * 2 + 1] as i32 * side_y / 255;
        let x1 = r.x + pad + sample.xy[i * 2] as i32 * side_x / 255;
        let y1 = r.y + pad + sample.xy[i * 2 + 1] as i32 * side_y / 255;
        canvas_line_aa(
            c,
            x0 as f32,
            y0 as f32,
            x1 as f32,
            y1 as f32,
            ui_px(3.0) as f32,
            0,
        );
    }
    let clip = rect_intersect(r, c.clip);
    for y in clip.y..clip.y + clip.h {
        for x in clip.x..clip.x + clip.w {
            let i = (y * c.stride + x) as usize;
            c.px[i] = if c.px[i] < 128 { 0 } else { 255 };
        }
    }
    c.clip = old_clip;
}

fn draw_gestures_modal(a: &mut App) {
    let group = lib(a).gestures_group;
    if group >= 0 {
        draw_bind_group(a, GESTURES, group as usize);
    } else {
        draw_bind_sheet(a, GESTURES);
    }
    if lib(a).record_action != 0 {
        hits_clear(&mut lib(a).hits);
        draw_record_card(a);
    } else if lib(a).gestures_reset_confirm {
        hits_clear(&mut lib(a).hits);
        draw_gesture_reset_card(a);
    }
}

fn draw_gesture_reset_card(a: &mut App) {
    let count = gesturemap_shapes(&a.gestures).len() as i64;
    let card = sheet_rect(a.w, a.h, ui_px(620.0));
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let in_ = ui_sheet(c, card, tr("Reset gestures?"));
    let mut prompt = String::new();
    // TRANSLATORS: confirmation before Reset gestures removes the recorded finger shapes.
    csnprintf!(
        &mut prompt,
        256,
        ng_(
            "Reset gestures? %d finger shape will be deleted.",
            "Reset gestures? %d finger shapes will be deleted.",
            count
        ),
        count
    );
    font_draw_centered(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(in_.x, in_.y, in_.w, ui_px(180.0)),
        &prompt,
        0,
    );
    let gap = ui_px(30.0);
    let bw = (in_.w - gap) / 2;
    let y = in_.y + in_.h - ui_px(140.0);
    let reset = Rect::new(in_.x, y, bw, ui_px(140.0));
    let cancel = Rect::new(reset.x + bw + gap, y, in_.w - bw - gap, ui_px(140.0));
    ui_button(c, reset, IconKind::Trash, tr("Reset"), ButtonStyle::Primary);
    ui_button(c, cancel, IconKind::Close, tr("Cancel"), ButtonStyle::Plain);
    hits_add(&mut s.hits, reset, H_GESTURES_RESET_CONFIRM, 0);
    hits_add(&mut s.hits, cancel, H_GESTURES_RESET_CANCEL, 0);
}

fn record_card_rect(a: &App) -> Rect {
    let w = ui_px(1400.0).min(a.w - ui_px(80.0));
    let h = ui_px(1000.0).min(a.h - ui_px(80.0));
    Rect::new((a.w - w) / 2, (a.h - h) / 2, w, h)
}

fn record_line_rect(a: &App) -> Rect {
    let card = record_card_rect(a);
    let margin = ui_px(54.0);
    Rect::new(
        card.x + margin,
        card.y + ui_px(560.0),
        card.w - 2 * margin,
        ui_px(100.0),
    )
}

fn draw_record_line(a: &mut App) {
    if lib(a).record_pen.touching {
        lib(a).record_pen_redraw = true;
        return;
    }
    let r = record_line_rect(a);
    canvas_fill(&mut a.screen, r, 255);
    let line = lib(a).record_line.clone();
    ui_text(
        &mut a.screen,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        r,
        &line,
        2,
        false,
        0,
    );
    app_invalidate(a, r, RefreshMode::Ui);
    app_flush(a);
}

fn draw_record_card(a: &mut App) {
    let card = record_card_rect(a);
    let line_r = record_line_rect(a);
    let title_action = lib(a).record_action;
    let phase = lib(a).record_phase;
    let line = lib(a).record_line.clone();
    let key = lib(a).record_key;
    let samples = lib(a).record_samples;
    let nsamples = lib(a).record_sample_count;
    let (screen, lib) = (&mut a.screen, &mut a.lib);
    let s = lib.as_deref_mut().unwrap();
    let in_ = ui_sheet(screen, card, "");
    let title = {
        let mut text = String::new();
        // TRANSLATORS: the record card's title, about 1300 px wide, 64 px down to 34 px; %s is an action name.
        csnprintf!(
            &mut text,
            256,
            tr("Do the gesture for %s"),
            action_label(title_action)
        );
        text
    };
    let top = in_.y + ui_px(12.0);
    ui_text(
        screen,
        FontFace::Bold,
        ui_px(64.0) as f32,
        ui_small_size(),
        Rect::new(in_.x, top, in_.w, ui_px(76.0)),
        &title,
        1,
        true,
        0,
    );
    ui_text(
        screen,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        Rect::new(in_.x, top + ui_px(82.0), in_.w, ui_px(90.0)),
        // TRANSLATORS: record card instructions, about 1312 px at 44 px or two lines at 34 px, upright and sideways.
        tr("Draw a shape with one finger, swipe or tap with two fingers, or use the pen's button."),
        2,
        false,
        0,
    );

    let slot = ui_px(260.0);
    let slot_gap = ui_px(40.0);
    let slots_w = 3 * slot + 2 * slot_gap;
    let slots_x = card.x + (card.w - slots_w) / 2;
    let slots_y = top + ui_px(190.0);
    if let Some(k) = key {
        let mut name = String::new();
        gesture_key_label(k, &mut name);
        font_draw_wrapped(
            screen,
            FontFace::Bold,
            ui_px(64.0) as f32,
            Rect::new(in_.x, slots_y, in_.w, slot),
            &name,
            2, 0, true, 0,
        );
    } else {
        for i in 0..3 {
            let r = Rect::new(slots_x + i as i32 * (slot + slot_gap), slots_y, slot, slot);
            canvas_fill(screen, r, 255);
            canvas_dash_rect(screen, r, ui_px(3.0), ui_px(12.0), ui_px(9.0), ui_px(2.0));
            let mut number = String::new();
            csnprintf!(&mut number, 8, "%d", i as i32 + 1);
            font_draw_centered(
                screen,
                FontFace::Regular,
                ui_px(34.0) as f32,
                r,
                &number,
                120,
            );
            if i < nsamples {
                let mut g = FsGesture::default();
                fs_prepare(&mut g, &samples[..=i]);
                let thumb = Rect::new(
                    r.x + ui_px(20.0),
                    r.y + ui_px(20.0),
                    r.w - ui_px(40.0),
                    r.h - ui_px(40.0),
                );
                draw_shape_thumb(screen, thumb, &g);
            }
        }
    }
    ui_text(
        screen,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        line_r,
        &line,
        2,
        false,
        0,
    );
    // TRANSLATORS: the record card's footer, about 1320 px at 34 px; shape recordings need two turns or a loop.
    font_draw_centered(
        screen,
        FontFace::Regular,
        ui_small_size(),
        Rect::new(in_.x, card.y + card.h - ui_px(230.0), in_.w, ui_px(60.0)),
        tr("One stroke, without lifting. Two turns or a loop."),
        0,
    );
    let by = card.y + card.h - ui_px(150.0);
    let gap = ui_px(20.0);
    let bw = (in_.w - 2 * gap) / 3;
    let use_r = Rect::new(in_.x, by, bw, ui_px(110.0));
    let retry_r = Rect::new(in_.x + bw + gap, by, bw, ui_px(110.0));
    let cancel_r = Rect::new(
        retry_r.x + bw + gap,
        by,
        in_.w - 2 * bw - 2 * gap,
        ui_px(110.0),
    );
    // TRANSLATORS: the record card's buttons; Use saves the recording, Try again clears this
    // attempt, and Cancel returns to the Gestures sheet.
    ui_button(
        screen,
        use_r,
        IconKind::Check,
        tr("Use"),
        if phase == RecordPhase::Ready {
            ButtonStyle::Primary
        } else {
            ButtonStyle::Disabled
        },
    );
    ui_button(
        screen,
        retry_r,
        IconKind::Undo,
        tr("Try again"),
        ButtonStyle::Plain,
    );
    ui_button(
        screen,
        cancel_r,
        IconKind::Close,
        tr("Cancel"),
        ButtonStyle::Plain,
    );
    hits_add(&mut s.hits, use_r, H_RECORD_USE, 0);
    hits_add(&mut s.hits, retry_r, H_RECORD_RETRY, 0);
    hits_add(&mut s.hits, cancel_r, H_RECORD_CANCEL, 0);
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
    csnprintf!(
        out,
        cap,
        tr("Automatic (%s)"),
        i18n_language_name(i18n_find(i18n_auto())).unwrap_or("(null)")
    );
}

fn draw_settings(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let mut lang = String::new();
    let mut lang_line = String::new();
    if language_picked(a) < 0 {
        language_auto_label(&mut lang, 96);
    } else {
        csnprintf!(
            &mut lang,
            96,
            "%s",
            i18n_language_name(language_picked(a)).unwrap_or("(null)")
        );
    }
    let App {
        screen: c,
        lib,
        settings,
        ..
    } = a;
    let s = lib.as_deref_mut().unwrap();
    let card = sheet_rect(aw, ah, ui_px(1820.0));
    let in_ = ui_sheet(c, card, tr("Settings"));
    let rh = ui_px(140.0);
    let gap = ui_px(24.0);
    let mut y = in_.y;
    let r1 = Rect::new(in_.x, y, in_.w, rh);
    ui_button(c, r1, IconKind::None, "", ButtonStyle::Plain);
    ui_icon(
        c,
        IconKind::Tune,
        Rect::new(
            r1.x + ui_px(20.0),
            r1.y + ui_px(15.0),
            rh - ui_px(30.0),
            rh - ui_px(30.0),
        ),
        0,
    );
    // TRANSLATORS: the rows of Settings: buttons with an icon (1470 px at 44 px in bold), then
    // lines a tap changes (1530 px at 44 px) and checkboxes (1548 px at 44 px, or two lines at
    // 34 px). Sideways they are 80 px wider.
    let pens = tr("Pen sensitivity (pressure + tilt per tool)");
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(r1.x + rh + ui_px(10.0), r1.y, r1.w - rh - ui_px(30.0), rh),
        pens,
        0,
    );
    hits_add(&mut s.hits, r1, H_SET_PENS, 0);
    y += rh + gap;
    let r2 = Rect::new(in_.x, y, in_.w, rh);
    ui_button(c, r2, IconKind::None, "", ButtonStyle::Plain);
    ui_icon(
        c,
        IconKind::Target,
        Rect::new(
            r2.x + ui_px(20.0),
            r2.y + ui_px(15.0),
            rh - ui_px(30.0),
            rh - ui_px(30.0),
        ),
        0,
    );
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
    ui_icon(
        c,
        IconKind::Brightness,
        Rect::new(
            r2b.x + ui_px(20.0),
            r2b.y + ui_px(15.0),
            rh - ui_px(30.0),
            rh - ui_px(30.0),
        ),
        0,
    );
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(
            r2b.x + rh + ui_px(10.0),
            r2b.y,
            r2b.w - rh - ui_px(30.0),
            rh,
        ),
        tr("Display: brightness, warmth, invert"),
        0,
    );
    hits_add(&mut s.hits, r2b, H_SET_DISPLAY, 0);
    y += rh + gap;
    let rt = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    canvas_fill(c, rt, 255);
    ui_icon(
        c,
        IconKind::Castle,
        Rect::new(rt.x, rt.y + ui_px(15.0), ui_px(80.0), ui_px(80.0)),
        0,
    );
    // TRANSLATORS: the style Grimoire is drawn in, and the other one a tap changes to.
    let mut style = tr("Style: Gothic (tap for Classic)");
    if ui_theme() == UiTheme::Classic {
        style = tr("Style: Classic (tap for Gothic)");
    }
    font_draw_left(
        c,
        FontFace::Regular,
        ui_text_size(),
        Rect::new(rt.x + ui_px(110.0), rt.y, rt.w - ui_px(110.0), rt.h),
        style,
        0,
    );
    hits_add(&mut s.hits, rt, H_SET_THEME, 0);
    y += ui_px(130.0);
    let r3 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    let mut row = String::new();
    // TRANSLATORS: %s is the folder on the Kindle's storage (seen over USB) that gets a PDF of
    // every notebook.
    csnprintf!(
        &mut row,
        256,
        tr("Mirror PDFs to %s (USB)"),
        "grimoire/PDFs"
    );
    ui_checkbox(c, r3, &row, settings.mirror_pdf);
    hits_add(&mut s.hits, r3, H_SET_MIRROR, 0);
    y += ui_px(130.0);
    let r4 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    // What holding the pen's side button does while writing (settings.ini side_button 0 to 3).
    static BTN_ROWS: [&str; 4] = [
        // TRANSLATORS: what holding the pen's button while writing does; a tap moves to the next.
        n_("Holding the pen button while writing does nothing (tap to change)"),
        n_("Holding the pen button while writing erases (tap to change)"),
        n_("Holding the pen button while writing highlights (tap to change)"),
        // TRANSLATORS: the fourth choice of the same row: while the pen's button is held, the pen
        // draws white ink (which takes away the ink under it and keeps the paper's lines). 1530 px
        // at 44 px, sideways 1610, or two lines at 34 px.
        n_("Holding the pen button while writing draws white (tap to change)"),
    ];
    canvas_fill(c, r4, 255);
    ui_icon(
        c,
        IconKind::Highlighter,
        Rect::new(r4.x, r4.y + ui_px(15.0), ui_px(80.0), ui_px(80.0)),
        0,
    );
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
    ui_checkbox(
        c,
        r5,
        tr("Turn with the Kindle (portrait and landscape)"),
        settings.follow_rotation,
    );
    hits_add(&mut s.hits, r5, H_SET_ROTATE, 0);
    y += ui_px(130.0);
    let r6 = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    ui_checkbox(
        c,
        r6,
        tr("Pinch with two fingers to zoom"),
        settings.pinch_zoom,
    );
    hits_add(&mut s.hits, r6, H_SET_PINCH, 0);
    y += ui_px(130.0);
    // The last row the sheet has room for: sideways in Gothic 18 px are left above Done.
    let rl = Rect::new(in_.x, y, in_.w, ui_px(110.0));
    // TRANSLATORS: a row of Settings, with the choice: "Automatic (English)", or a language's name.
    csnprintf!(&mut lang_line, 160, tr("Language: %s"), &lang);
    canvas_fill(c, rl, 255);
    ui_icon(
        c,
        IconKind::Language,
        Rect::new(rl.x, rl.y + ui_px(15.0), ui_px(80.0), ui_px(80.0)),
        0,
    );
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
    font_draw_left(
        c,
        FontFace::Regular,
        ui_small_size(),
        Rect::new(in_.x, y, in_.w, ui_px(60.0)),
        &about,
        60,
    );
    // (The data's path is a name: a long one is cut short, as names are. tests/test_i18n.c checks
    // that the label fits with the Kindle's path.)
    csnprintf!(&mut about, 320, tr("Data: %s"), &store_root());
    font_draw_left_name(
        c,
        FontFace::Regular,
        ui_small_size(),
        Rect::new(in_.x, y + ui_px(60.0), in_.w, ui_px(60.0)),
        &about,
        60,
    );
    let done = Rect::new(
        in_.x + in_.w - ui_px(380.0),
        in_.y + in_.h - ui_px(140.0),
        ui_px(380.0),
        ui_px(140.0),
    );
    ui_button(c, done, IconKind::None, tr("Done"), ButtonStyle::Primary);
    hits_add(&mut s.hits, done, H_CANCEL, 0);
    let ko = Rect::new(in_.x, done.y, ui_px(300.0), done.h);
    ui_button(c, ko, IconKind::Notebook, "KOReader", ButtonStyle::Plain);
    hits_add(&mut s.hits, ko, H_KOREADER, 0);
    // Controller, Gestures and Updates share the space between KOReader and Done.
    let gap = ui_px(30.0);
    let mid = done.x - ko.x - ko.w - 4 * gap;
    let third = mid / 3;
    let keys = Rect::new(ko.x + ko.w + gap, done.y, third, done.h);
    let gestures = Rect::new(keys.x + keys.w + gap, done.y, third, done.h);
    let upd = Rect::new(
        gestures.x + gestures.w + gap,
        done.y,
        mid - 2 * third,
        done.h,
    );
    // TRANSLATORS: Settings' bottom buttons, each a word under its icon, about 240 px at 34 px.
    // Controller opens the connected key/controller settings, Gestures opens gesture bindings,
    // Updates checks for a newer Grimoire.
    ui_button(
        c,
        keys,
        IconKind::Gamepad,
        tr("Controller"),
        ButtonStyle::Plain,
    );
    hits_add(&mut s.hits, keys, H_SHORTCUTS, 0);
    ui_button(
        c,
        gestures,
        IconKind::Gestures,
        tr("Gestures"),
        ButtonStyle::Plain,
    );
    hits_add(&mut s.hits, gestures, H_GESTURES, 0);
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
            let (upd, newer, latest, why) =
                (s.upd, upd_newer(s), s.upd_latest.clone(), s.upd_why.clone());
            if !avail {
                // TRANSLATORS: the Updates sheet's first line (about 1100 px at 44 px in bold): the version
                // running; then while it checks; then what the check found: a newer one (%s), or that this
                // one (%s) is the newest; or why it could not check.
                csnprintf!(
                    &mut status,
                    200,
                    "%s",
                    tr("Updates come through KPM, which is not here.")
                );
            } else if upd == UPD_CHECKING {
                csnprintf!(&mut status, 200, "%s", tr("Checking for updates..."));
            } else if upd == UPD_DONE && newer {
                csnprintf!(&mut status, 200, tr("Grimoire %s is available."), &latest);
            } else if upd == UPD_DONE {
                csnprintf!(
                    &mut status,
                    200,
                    tr("Grimoire %s is the newest."),
                    GRIMOIRE_VERSION
                );
            } else if upd == UPD_FAILED && why == "network" {
                csnprintf!(&mut status, 200, "%s", tr("Could not check. Is Wi-Fi on?"));
            } else if upd == UPD_FAILED && why == "no-kpm" {
                csnprintf!(
                    &mut status,
                    200,
                    "%s",
                    tr("Updates come through KPM, which is not here.")
                );
            } else if upd == UPD_FAILED {
                csnprintf!(&mut status, 200, "%s", tr("Could not check for updates."));
            } else {
                csnprintf!(
                    &mut status,
                    200,
                    tr("This is Grimoire %s."),
                    GRIMOIRE_VERSION
                );
            }
            // TRANSLATORS: (Updates sheet) which updates: betas too (the preview channel), or releases
            // only (stable); a tap changes it.
            csnprintf!(
                &mut channel,
                200,
                "%s",
                if upd_preview(a) {
                    tr("Updates: betas and releases (tap for releases only)")
                } else {
                    tr("Updates: releases only (tap for betas too)")
                }
            );
            // TRANSLATORS: (Updates sheet) installs the newer Grimoire (%s): Grimoire closes, KPM
            // installs it, and Grimoire starts again.
            csnprintf!(&mut now, 160, tr("Update to %s"), &latest);
            let can = avail && upd != UPD_CHECKING;
            let rows = [
                menu_row(IconKind::None, &status, H_NONE, 0, ButtonStyle::Disabled),
                menu_row(
                    IconKind::None,
                    &channel,
                    H_UPD_CHANNEL,
                    0,
                    if can {
                        ButtonStyle::Plain
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
                // TRANSLATORS: (Updates sheet) asks Grimoire's feed for a newer Grimoire (through KPM).
                menu_row(
                    IconKind::Update,
                    tr("Check for updates"),
                    H_UPD_CHECK,
                    0,
                    if can {
                        ButtonStyle::Plain
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
                menu_row(
                    IconKind::Download,
                    if newer { now.as_str() } else { tr("Update") },
                    H_UPD_NOW,
                    0,
                    if newer {
                        ButtonStyle::Primary
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
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
            let first = if s.scanning {
                tr("Searching... Put the controller in pairing mode.")
            } else if s.pairing {
                tr("Pairing...")
            } else {
                tr("Tap a controller to pair it.")
            };
            rows.push(menu_row(
                IconKind::None,
                first,
                H_NONE,
                0,
                ButtonStyle::Disabled,
            ));
            let mut i = 0;
            while i < s.nfound && rows.len() < 5 {
                let d = &s.found[i as usize];
                let mut name = String::new();
                csnprintf!(
                    &mut name,
                    BT_NAME_LEN + 40,
                    "%s",
                    if !d.name.is_empty() {
                        d.name.as_str()
                    } else {
                        d.address.as_str()
                    }
                );
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
                csnprintf!(
                    &mut name,
                    BT_NAME_LEN + 40,
                    tr("Forget %s"),
                    if !p.name.is_empty() {
                        p.name.as_str()
                    } else {
                        p.address.as_str()
                    }
                );
                rows.push(menu_row(
                    IconKind::Trash,
                    &name,
                    H_PAIR_FORGET,
                    i,
                    if s.pairing {
                        ButtonStyle::Disabled
                    } else {
                        ButtonStyle::Plain
                    },
                ));
                i += 1;
            }
            // TRANSLATORS: (Pair sheet) searches for controllers again.
            rows.push(menu_row(
                IconKind::None,
                tr("Search again"),
                H_PAIR_AGAIN,
                0,
                if s.scanning || s.pairing {
                    ButtonStyle::Disabled
                } else {
                    ButtonStyle::Plain
                },
            ));
            rows.push(menu_row(
                IconKind::Back,
                tr("Done"),
                H_BUTTONS_DONE,
                0,
                ButtonStyle::Plain,
            ));
            // TRANSLATORS: the Pair sheet's title (Settings > Controller > Pair a controller).
            draw_menu(a, tr("Pair a controller"), &rows);
        }
        Modal::Buttons => draw_bind_sheet(a, BUTTONS),
        Modal::Gestures => draw_gestures_modal(a),
        Modal::Shortcuts => {
            let mut count = String::new();
            let mut last = String::new();
            let mut status = String::new();
            // TRANSLATORS: (Controller sheet) how many shortcuts grimoire/shortcuts.ini (and controller.ini)
            // hold, and how many of their lines were refused.
            csnprintf!(
                &mut count,
                160,
                tr("Mappings: %d; rejected: %d"),
                a.shortcuts.count,
                a.shortcuts.rejected
            );
            if a.last_shortcut_code != 0 {
                csnprintf!(
                    &mut last,
                    160,
                    tr("Last input: %s %d; modifiers: %d"),
                    if a.last_shortcut_gamepad {
                        "pad"
                    } else {
                        "key"
                    },
                    a.last_shortcut_code,
                    a.last_shortcut_mods
                );
            } else {
                csnprintf!(
                    &mut last,
                    160,
                    "%s",
                    tr("Press a connected key or controller button")
                );
            }
            let (code, mods, gamepad) = (
                a.last_shortcut_code,
                a.last_shortcut_mods,
                a.last_shortcut_gamepad,
            );
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
                csnprintf!(
                    &mut status,
                    160,
                    "%s",
                    tr("Bluetooth did not start (see grimoire.log)")
                );
            }
            let on = a.settings.shortcuts_enabled;
            let present = bt_present();
            let bt_up = bs == BtState::Owned || bs == BtState::External;
            let mut reload = String::new();
            // TRANSLATORS: (Controller sheet) reads grimoire/shortcuts.ini (and controller.ini) again: how
            // many shortcuts they hold, and how many of their lines were refused.
            csnprintf!(
                &mut reload,
                160,
                tr("Reload shortcuts.ini (%d, %d refused)"),
                a.shortcuts.count,
                a.shortcuts.rejected
            );
            let rows = [
                menu_row(
                    if on { IconKind::Check } else { IconKind::None },
                    tr("Enable shortcuts"),
                    H_SHORTCUT_ENABLE,
                    0,
                    ButtonStyle::Plain,
                ),
                // TRANSLATORS: (Controller sheet) starts the Bluetooth helper with Grimoire, for a
                // Bluetooth controller; without the helper unpacked in grimoire/bluetooth, the second.
                menu_row(
                    if a.settings.controller && present {
                        IconKind::Check
                    } else {
                        IconKind::None
                    },
                    if present {
                        tr("Bluetooth controller")
                    } else {
                        tr("No Bluetooth helper in grimoire/bluetooth")
                    },
                    H_BT_TOGGLE,
                    0,
                    if present && on {
                        ButtonStyle::Plain
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
                // TRANSLATORS: (Controller sheet) opens the Pair sheet: find a Bluetooth controller and pair it.
                menu_row(
                    IconKind::Plus,
                    tr("Pair a controller..."),
                    H_PAIR,
                    0,
                    if bt_up && on {
                        ButtonStyle::Plain
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
                menu_row(IconKind::None, &status, H_NONE, 0, ButtonStyle::Disabled),
                // TRANSLATORS: (Controller sheet) opens the Buttons sheet: which controller button does what.
                menu_row(
                    IconKind::None,
                    tr("Buttons..."),
                    H_BUTTONS,
                    0,
                    if on {
                        ButtonStyle::Plain
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
                menu_row(
                    IconKind::None,
                    &reload,
                    H_SHORTCUT_RELOAD,
                    0,
                    ButtonStyle::Plain,
                ),
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
            let if_found = if found {
                ButtonStyle::Plain
            } else {
                ButtonStyle::Disabled
            };
            let rows = [
                menu_row(
                    IconKind::Notebook,
                    if found {
                        tr("Install / update Grimoire plugin")
                    } else {
                        tr("KOReader not found")
                    },
                    H_KO_ITEM,
                    0,
                    if_found,
                ),
                menu_row(
                    IconKind::Trash,
                    tr("Remove Grimoire plugin"),
                    H_KO_ITEM,
                    -1,
                    if installed {
                        ButtonStyle::Plain
                    } else {
                        ButtonStyle::Disabled
                    },
                ),
                menu_row(
                    if koreader_item_installed(1) {
                        IconKind::Check
                    } else {
                        IconKind::None
                    },
                    tr("Disable diagonal screenshot gestures"),
                    H_KO_ITEM,
                    1,
                    if_found,
                ),
                menu_row(
                    if koreader_item_installed(2) {
                        IconKind::Check
                    } else {
                        IconKind::None
                    },
                    tr("Square folder corners"),
                    H_KO_ITEM,
                    2,
                    if_found,
                ),
                menu_row(
                    if koreader_item_installed(3) {
                        IconKind::Check
                    } else {
                        IconKind::None
                    },
                    tr("Terminess menu font"),
                    H_KO_ITEM,
                    3,
                    if_found,
                ),
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
                menu_row(
                    IconKind::Notebook,
                    tr("Open"),
                    H_MORE_OPEN,
                    0,
                    ButtonStyle::Plain,
                ),
                menu_row(
                    IconKind::Rename,
                    tr("Rename"),
                    H_MORE_RENAME,
                    0,
                    ButtonStyle::Plain,
                ),
                menu_row(
                    IconKind::FolderMove,
                    tr("Move to folder"),
                    H_MORE_MOVE,
                    0,
                    ButtonStyle::Plain,
                ),
                menu_row(IconKind::Download, &pdf, H_MORE_PDF, 0, ButtonStyle::Plain),
                menu_row(
                    IconKind::Trash,
                    tr("Delete"),
                    H_MORE_DELETE,
                    0,
                    ButtonStyle::Plain,
                ),
                menu_row(
                    IconKind::Close,
                    tr("Cancel"),
                    H_CANCEL,
                    0,
                    ButtonStyle::Plain,
                ),
            ];
            let title = name_label(lib(a).books[it.nb].name.as_str());
            draw_menu(a, &title, &rows);
        }
        Modal::FolderMore => {
            let Some(it) = it else { return };
            // TRANSLATORS: a folder's menu (1470 px at 44 px in bold); only an empty folder can go.
            let rows = [
                menu_row(
                    IconKind::Rename,
                    tr("Rename folder"),
                    H_FMORE_RENAME,
                    0,
                    ButtonStyle::Plain,
                ),
                menu_row(
                    IconKind::Trash,
                    if it.count != 0 || it.has_children { tr("Remove folder (move its notebooks out first)") } else { tr("Remove folder") },
                    H_FMORE_REMOVE,
                    0,
                    if it.count != 0 || it.has_children { ButtonStyle::Disabled } else { ButtonStyle::Plain },
                ),
                menu_row(IconKind::Close,tr("Cancel"),H_CANCEL,0,ButtonStyle::Plain,
                ),
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
            rows.push(menu_row(
                IconKind::Library,
                tr("Library (top level)"),
                H_MOVE_TOP,
                0,
                if !curf.is_empty() { ButtonStyle::Plain } else { ButtonStyle::Selected },
            ));
            let mut f = s.folder_move_page * 6;
            while f < s.folders.len() && f < (s.folder_move_page + 1) * 6 {
                let st = if curf != s.folders[f].as_str() { ButtonStyle::Plain } else { ButtonStyle::Selected };
                rows.push(menu_row(IconKind::Folder, &s.folders[f], H_MOVE_TO, f as i32, st));
                f += 1;
            }
            if s.folder_move_page > 0 {
                rows.push(menu_row(IconKind::Back, tr("Previous folders"), H_MOVE_PREV, 0, ButtonStyle::Plain));
            }
            if f < s.folders.len() {
                rows.push(menu_row(IconKind::None, tr("More folders"), H_MOVE_NEXT, 0, ButtonStyle::Plain));
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
            csnprintf!(
                &mut del,
                256,
                tr("Delete (moves it to %s)"),
                "grimoire/trash"
            );
            let rows = [
                menu_row(IconKind::Trash, &del, H_DELETE_YES, 0, ButtonStyle::Primary),
                menu_row(
                    IconKind::Close,
                    tr("Keep it"),
                    H_CANCEL,
                    0,
                    ButtonStyle::Plain,
                ),
            ];
            draw_menu(a, &title, &rows);
        }
        Modal::Language => {
            // Automatic, then every language by its own name; a pick applies at once.
            let mut rows: Vec<MenuRow> = Vec::with_capacity(12);
            let cur = language_picked(a);
            let mut auto_label = String::new();
            language_auto_label(&mut auto_label, 96);
            rows.push(menu_row(
                IconKind::Language,
                &auto_label,
                H_LANG_PICK,
                -1,
                if cur < 0 {
                    ButtonStyle::Selected
                } else {
                    ButtonStyle::Plain
                },
            ));
            let mut i = 0;
            while i < i18n_language_count() && rows.len() < 11 {
                let name = i18n_language_name(i).unwrap_or("(null)");
                rows.push(menu_row(
                    IconKind::None,
                    name,
                    H_LANG_PICK,
                    i,
                    if cur == i {
                        ButtonStyle::Selected
                    } else {
                        ButtonStyle::Plain
                    },
                ));
                i += 1;
            }
            rows.push(menu_row(
                IconKind::Close,
                tr("Cancel"),
                H_LANG_BACK,
                0,
                ButtonStyle::Plain,
            ));
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
    let t = ui_toast_rect(
        Rect::new(0, 0, a.w, a.h),
        if a.toast_visible {
            a.toast.as_str()
        } else {
            ""
        },
    );
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
    if lib(a).settings_return_editor && a.scr==ScreenId::Editor {
        lib(a).settings_painting=true;
        crate::editor::editor_settings_background(a);
        lib(a).settings_painting=false;
    } else {draw_base(a);}
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
    if lib(a).settings_return_editor {
        if lib(a).modal==Modal::Settings {let changed=lib(a).settings_editor_fit!=a.settings.landscape_fit;lib(a).settings_return_editor=false;lib(a).modal=Modal::None;library_leave(a);if changed {editor_fit_changed(a,true);}crate::editor::editor_settings_background(a);app_invalidate_all(a,RefreshMode::Ui);}
        else {lib(a).modal=Modal::Settings;redraw(a,RefreshMode::Ui);}
        return;
    }
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
    qlog!(
        "library: %s: the new notebook's template could not be used: %s",
        &nb.id,
        why
    );
    false
}

fn create_notebook(a: &mut App) {
    let mut nb = Box::<NotebookInfo>::default();
    let s = lib(a);
    if store_create(
        s.new_name.as_str(),
        s.cur_folder.as_str(),
        s.new_paper,
        &mut nb,
    ) == 0
    {
        s.modal = Modal::None;
        let tpl_ok = s.new_tpl.is_empty() || start_on_template(&mut nb, s.new_tpl.as_str());
        let id = nb.id.as_str().to_string();
        drop(nb);
        editor_open_notebook(a, &id);
        if !tpl_ok {
            let mut msg = String::new();
            csnprintf!(
                &mut msg,
                256,
                tr("Could not use that template (see %s)."),
                "grimoire.log"
            );
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
            KbPurpose::NewFolder | KbPurpose::MoveNewFolder => {
                let parent = lib(a).cur_folder;
                let mut path = FixedStr::new();
                if !store_folder_join(parent.as_str(), &txt, &mut path) || store_add_folder(path.as_str()) != 0 {
                    app_toast(a, tr("Could not create the folder. Use a shorter name without a slash."), 3500);
                } else if kbp == KbPurpose::MoveNewFolder {
                    if let Some(it) = it.filter(|it| !it.is_folder) {
                        let id = lib(a).books[it.nb].id.as_str().to_string();
                        if store_set_folder(&id, path.as_str()) == 0 {
                            mirror_follow(a, &id, false);
                        } else {
                            app_toast(a, tr("Could not move the notebook."), 3000);
                        }
                    }
                }
            }
            KbPurpose::RenameFolder => {
                if let Some(it) = it.filter(|it| it.is_folder) {
                    let mut dest = FixedStr::new();
                    if !store_folder_join(store_folder_parent(it.folder.as_str()), &txt, &mut dest)
                        || store_rename_folder(it.folder.as_str(), dest.as_str()) != 0
                    {
                        app_toast(a, tr("Could not rename the folder. Check the name and destination."), 3500);
                    } else {
                        let ids: Vec<String> = lib(a)
                            .books
                            .iter()
                            .filter(|nb| store_folder_contains(it.folder.as_str(), nb.folder.as_str()))
                            .map(|nb| nb.id.as_str().to_string())
                            .collect();
                        for id in ids {
                            mirror_follow(a, &id, false);
                        }
                        export_mirror_prune_folder(it.folder.as_str(), &app_mirror_dir());
                    }
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
            let parent = store_folder_parent(s.cur_folder.as_str()).to_string();
            s.cur_folder.set(&parent);
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
            s.new_paper = PAPER_NARROW_NM;
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
            let m = if s.items[h.arg as usize].is_folder {
                Modal::FolderMore
            } else {
                Modal::More
            };
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
            open_kb(
                a,
                KbPurpose::NewName,
                c_("keyboard", "Name"),
                name.as_bytes(),
            );
        }
        H_NEW_CREATE => create_notebook(a),
        H_CANCEL => close_modal(a),
        H_GESTURES => open_gestures(a),
        H_GESTURE_GROUP => {
            let s = lib(a);
            bind_group_set(GESTURES, s, clampi(h.arg, 0, N_GROUPS as i32 - 1));
            s.gestures_learning = SHORTCUT_NONE;
            redraw(a, RefreshMode::Ui);
        }
        H_GESTURE_ROW => {
            let s = lib(a);
            if let Some(row) = binding_row_at(GESTURES, s.gestures_group.max(0) as usize, h.arg.max(0) as usize) {
                if s.record_action == row.act[0] {
                    close_record_card(a);
                } else {
                    (GESTURES.learn)(a, row.act[0]);
                    lib(a).record_next_action = row.act[1];
                }
            }
        }
        H_GESTURES_MASTER => {
            app_global_action(a, SHORTCUT_GESTURES, ActionSrc::Touch);
            input_touch_context(TgView { gestures_on: a.settings.gestures, ..TgView::default() });
            redraw(a, RefreshMode::Ui);
        }
        H_GESTURES_RESET => { (GESTURES.reset)(a); }
        H_GESTURES_RESET_CONFIRM => { reset_gesture_map(a); }
        H_GESTURES_RESET_CANCEL => {
            lib(a).gestures_reset_confirm = false;
            redraw(a, RefreshMode::Ui);
        }
        H_GESTURES_DONE => {
            if lib(a).gestures_group >= 0 {
                lib(a).gestures_group = -1;
                redraw(a, RefreshMode::Ui);
            } else {
                open_modal(a, Modal::Settings);
            }
        }
        H_RECORD_USE => use_recording(a),
        H_RECORD_RETRY => { let action = lib(a).record_action; open_record_card(a, action); }
        H_RECORD_CANCEL => close_record_card(a),
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
                qlog!(
                    "settings: shortcuts %d, controller %d",
                    a.settings.shortcuts_enabled,
                    a.settings.controller
                );
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
            s.buttons_group = clampi(h.arg, 0, N_GROUPS as i32 - 1);
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
            qlog!(
                "settings: shortcuts %d, controller %d",
                a.settings.shortcuts_enabled,
                a.settings.controller
            );
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
            let enabled = if h.arg < 0 {
                false
            } else {
                item == 0 || !koreader_item_installed(item)
            };
            if koreader_item_set(item, enabled) == 0 {
                app_toast(a, tr("Updated. Restart KOReader to apply."), 5000);
            } else {
                app_toast(
                    a,
                    tr("Could not update KOReader. A different patch or a storage error?"),
                    7000,
                );
            }
            redraw(a, RefreshMode::Ui);
        }
        H_SET_PENS => {
            if !lib(a).settings_return_editor {lib(a).modal=Modal::None;}
            let tool = a.settings.current_tool;
            pensettings_open(a, tool);
        }
        H_SET_CALIB => {
            if !lib(a).settings_return_editor {lib(a).modal=Modal::None;}
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
            a.settings.theme = if a.settings.theme == UiTheme::Classic as i32 {
                UiTheme::Gothic as i32
            } else {
                UiTheme::Classic as i32
            };
            ui_set_theme(if a.settings.theme == UiTheme::Classic as i32 {
                UiTheme::Classic
            } else {
                UiTheme::Gothic
            });
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
            let code = if h.arg >= 0 {
                i18n_language_code(h.arg)
            } else {
                Some("auto")
            };
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
                open_kb(
                    a,
                    KbPurpose::Rename,
                    c_("keyboard", "Name"),
                    name.as_bytes(),
                );
            }
        }
        H_MORE_MOVE => {
            lib(a).folder_move_page = 0;
            open_modal(a, Modal::Move);
        }
        H_MOVE_PREV | H_MOVE_NEXT => {
            let s = lib(a);
            if h.id == H_MOVE_PREV {
                s.folder_move_page = s.folder_move_page.saturating_sub(1);
            } else if (s.folder_move_page + 1) * 6 < s.folders.len() {
                s.folder_move_page += 1;
            }
            redraw(a, RefreshMode::Ui);
        }
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
                open_kb(a, KbPurpose::RenameFolder, c_("keyboard", "Folder"), store_folder_leaf(it.folder.as_str()).as_bytes());
            }
        }
        H_FMORE_REMOVE => {
            if it.is_some_and(|it| it.count != 0 || it.has_children) {
                return;
            }
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
                if store_set_folder(&id, &folder) == 0 {
                    mirror_follow(a, &id, false);
                } else {
                    app_toast(a, tr("Could not move the notebook."), 3000);
                }
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
    s.gestures_group = -1;
}

pub fn library_free(a: &mut App) {
    let Some(s) = a.lib.as_deref_mut() else {
        return;
    };
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
        s.settings_return_editor = false;
    }
    s.pressed = None;
    reload(a);
    let record = library_record_active(a);
    input_touch_context(TgView { gestures_on: record || a.settings.gestures, record, ..TgView::default() });
    redraw(a, RefreshMode::Full);
}

pub fn library_leave(a: &mut App) {
    let s = lib(a);
    s.pressed = None;
    s.record_touch_active = false;
    s.record_previous_lift = None;
    s.record_pb = PenButtons::default();
    s.record_pen = PbPen::default();
    s.record_pen_pointer = false;
    s.record_pen_capture = false;
    s.record_pen_redraw = false;
    fs_abort(&mut s.record_stroke);
    input_touch_context(TgView { gestures_on: a.settings.gestures, record: false, ..TgView::default() });
}

pub fn library_event(a: &mut App, ev: &InEvent) {
    if library_record_active(a) && record_event(a, ev) {
        return;
    }
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
        } else if r == DmResult::Refit {
            // (the editor, not open here, lays its page out again when it is next entered)
            editor_fit_changed(a, false);
            redraw(a, RefreshMode::Ui);
            editor_fit_toast(a);
        } else if !rect_is_empty(d) {
            app_invalidate(
                a,
                d,
                if ph == 2 {
                    RefreshMode::Ui
                } else {
                    RefreshMode::Fast
                },
            );
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
    if library_record_active(a) {
        // A queued release/contact must reach the model before a wall-clock tick.
        if a.input_drained {
            let config = PbConfig { waits:[true,true,false], ..a.pb_config };
            for _ in 0..3 {
                let out = penbutton_tick(&mut lib(a).record_pb, &config, now);
                if out == PbOut::None { break; }
                record_pen_output(a, out);
            }
        }
        let s = lib(a);
        if !s.record_attempted && !s.record_touch_active && now.wrapping_sub(s.record_started) >= LEARN_MS {
            close_record_card(a);
        } else if s.record_touch_active {
            let state = fs_tick(&mut s.record_stroke, now);
            if state == FsState::Drawing && !s.record_touch_origin {
                s.record_touch_origin = true;
                s.record_attempted = true;
                if let Some(h) = s.pressed.take() {
                    canvas_invert(&mut a.screen, h.r);
                    app_invalidate(a, h.r, RefreshMode::Ui);
                }
            }
            draw_record_trail(a, now);
        }
    }
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
    let s = lib(a);
    if s.modal == Modal::Gestures && s.gestures_group >= 0 && s.record_action == SHORTCUT_NONE
        && !s.gestures_reset_confirm && s.pressed.is_some_and(|p| p.id == H_GESTURE_ROW)
        && !s.press.moved && now.wrapping_sub(s.pressed_at) >= ROW_HOLD_MS
    {
        let row = s.pressed.take().unwrap().arg;
        if let Some(row) = binding_row_at(GESTURES, s.gestures_group as usize, row as usize) {
            (GESTURES.forget)(a, row);
            redraw(a, RefreshMode::Ui);
        }
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
    (
        v.clamp(libc::c_long::MIN as i128, libc::c_long::MAX as i128) as i64,
        i,
    )
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
        csnprintf!(
            out,
            cap,
            "%s",
            ui_name_label(
                if it.is_folder { store_folder_leaf(it.folder.as_str()) } else { s.books[it.nb].name.as_str() },
                &mut label,
                LABEL_CAP
            )
        );
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
        csnprintf!(
            out,
            cap,
            "%d",
            (sum + np / 2).checked_div(np).map_or(255, |m| m as i32)
        );
    } else if !s.thumbs[it.nb].px.is_empty() {
        csnprintf!(
            out,
            cap,
            "%s",
            if s.thumb_picture[it.nb] {
                "picture"
            } else {
                "thumb"
            }
        );
    } else {
        // (as paper_preview draws it)
        csnprintf!(
            out,
            cap,
            "paper %u",
            if !paper_is_known(k) || paper_is_image(k) {
                PAPER_BLANK.0
            } else {
                k.0
            }
        );
    }
    0
}

/// The headless runner's "library.*" keys, asked without the prefix (app_probe's contract);
/// "templates.*" keys reach it whole (docs/SIM.md).
// The script runner's "library.*" keys (app_probe forwards them without the prefix).
pub fn library_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    static SHEETS: [&str; 18] = [
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
        "gestures",    // MOD_GESTURES
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
    } else if key == "gestures_group" {
        let group = if s.modal == Modal::Gestures && s.gestures_group >= 0 {
            ACTION_GROUPS.get(s.gestures_group as usize).map_or("", |g| g.probe)
        } else { "" };
        csnprintf!(out, cap, "%s", group);
    } else if key == "gestures.learning" {
        let name = if s.modal == Modal::Gestures && s.gestures_learning != SHORTCUT_NONE {
            shortcut_action_name(s.gestures_learning).unwrap_or("0")
        } else { "0" };
        csnprintf!(out, cap, "%s", name);
    } else if let Some(row) = key.strip_prefix("gestures.") {
        let Ok(row) = row.parse::<usize>() else { return -1; };
        if s.modal != Modal::Gestures || row == 0 { return -1; }
        if s.gestures_group < 0 {
            let rows = bind_menu_rows(a, GESTURES);
            let Some(row) = rows.get(row - 1) else { return -1; };
            csnprintf!(out, cap, "%s", &row.label);
            return 0;
        }
        let Some(row) = binding_row_at(GESTURES, s.gestures_group as usize, row - 1) else { return -1; };
        let mut text = String::new();
        bind_row_text(a, GESTURES, row, &mut text);
        csnprintf!(out, cap, "%s", &text);
    } else if key == "record.line" {
        csnprintf!(out, cap, "%s", if library_record_active(a) { &lib(a).record_line } else { "" });
    } else if key == "record" {
        if !library_record_active(a) {
            csnprintf!(out, cap, "0");
        } else if let Some(key) = lib(a).record_key {
            let mut name = String::new();
            gesturemap_key_name(key, &mut name);
            csnprintf!(out, cap, "pattern %s", &name);
        } else {
            let s = lib(a);
            match s.record_phase {
                RecordPhase::Checking => csnprintf!(out, cap, "checking"),
                RecordPhase::Ready => csnprintf!(out, cap, "ready"),
                RecordPhase::Shape => csnprintf!(out, cap, "shape %d", s.record_sample_count),
                _ => csnprintf!(out, cap, "wait"),
            };
        }
    } else if let Some(point) = key.strip_prefix("ui_at.") {
        let Some((x, y)) = point.split_once('.') else { return -1; };
        let (Ok(x), Ok(y)) = (x.parse::<i32>(), y.parse::<i32>()) else { return -1; };
        let hit = hits_find(&s.hits, x as f32, y as f32);
        let mut name = String::new();
        if let Some(hit) = hit {
            name.push_str(match hit.id {
                H_NEW=>"new",H_NEWFOLDER=>"new-folder",H_SETTINGS=>"settings",H_SHARE=>"share",H_EXIT=>"exit",
                H_SET_THEME=>"theme",H_NEW_CREATE=>"create",
                H_SET_PENS => "pen-feel", H_SET_CALIB => "calibrate", H_SET_DISPLAY => "display",
                H_KOREADER => "ko", H_SHORTCUTS => "controller", H_GESTURES => "gestures",
                H_UPDATES => "updates", H_CANCEL => "done", H_GESTURES_MASTER => "gestures.master",
                H_GESTURES_RESET => "gestures.reset", H_GESTURES_DONE => "done",
                H_GESTURES_RESET_CONFIRM => "gestures.reset.confirm",
                H_GESTURES_RESET_CANCEL => "gestures.reset.cancel",
                H_RECORD_USE => "record.use", H_RECORD_RETRY => "record.retry", H_RECORD_CANCEL => "record.cancel",
                H_GESTURE_GROUP => "gestures.group.", H_GESTURE_ROW => "gestures.row.",
                H_BUTTON_GROUP => "buttons.group.", H_BUTTON_ROW => "buttons.row.",
                H_BUTTONS_DONE => "done", H_BUTTONS_RESET => "buttons.reset",
                _ => "",
            });
            if matches!(hit.id, H_GESTURE_GROUP | H_BUTTON_GROUP) {
                if let Some(group) = ACTION_GROUPS.get(hit.arg as usize) { name.push_str(group.probe); }
            } else if matches!(hit.id, H_GESTURE_ROW | H_BUTTON_ROW) {
                name.push_str(&(hit.arg + 1).to_string());
            }
        }
        csnprintf!(out, cap, "%s", &name);
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
        csnprintf!(
            out,
            cap,
            "%s",
            if open {
                ACTION_GROUPS[s.buttons_group as usize].probe
            } else {
                ""
            }
        );
    } else if key == "update" {
        // The Updates sheet's check: "idle", "checking", "newer <version>", "newest <version>" or
        // "failed <why>".
        if s.upd == UPD_CHECKING {
            csnprintf!(out, cap, "checking");
        } else if s.upd == UPD_DONE {
            csnprintf!(
                out,
                cap,
                "%s %s",
                if upd_newer(s) { "newer" } else { "newest" },
                &s.upd_latest
            );
        } else if s.upd == UPD_FAILED {
            csnprintf!(out, cap, "failed %s", &s.upd_why);
        } else {
            csnprintf!(out, cap, "idle");
        }
    } else if key == "pair" {
        // The Pair sheet: "scanning", "pairing" or "idle", then the controllers found and paired.
        csnprintf!(
            out,
            cap,
            "%s %d %d",
            if s.scanning {
                "scanning"
            } else if s.pairing {
                "pairing"
            } else {
                "idle"
            },
            s.nfound,
            s.npaired
        );
    } else if key == "bluetooth" {
        static NAMES: [&str; 6] = [
            "stopped", "starting", "owned", "external", "failed", "stopping",
        ];
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
        csnprintf!(
            out,
            cap,
            "%s",
            if s.modal == Modal::Kb {
                s.kb.text.as_str()
            } else {
                ""
            }
        );
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
        return tplsheet_probe(
            if s.modal == Modal::Templates {
                &s.ts
            } else {
                &closed
            },
            rest,
            out,
            cap,
        );
    } else if key == "music" {
        // The layout on the Custom music sheet while it is open, else New notebook's custom paper.
        let mut m = MusicLayout::default();
        if s.modal == Modal::Music {
            app_probe_music(Some(&s.mb.m), out, cap);
        } else {
            app_probe_music(
                if music_decode(s.new_paper, &mut m) {
                    Some(&m)
                } else {
                    None
                },
                out,
                cap,
            );
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

    fn probe(a: &mut App, key: &str) -> String {
        let mut out = String::new();
        assert_eq!(library_probe(a, key, &mut out, 512), 0, "{key}");
        out
    }

    fn pen_record_app() -> App {
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, input_drained: true, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        #[cfg(feature = "sim")]
        {
            let (mut w, mut h) = (a.w, a.h);
            crate::sim_sdl::display_set_orientation(0, &mut w, &mut h);
        }
        library_init(&mut a);
        gesturemap_defaults(&mut a.gestures);
        app_gestures_sync(&mut a);
        let s = lib(&mut a);
        s.modal = Modal::Gestures;
        s.gestures_group = 0;
        s.gestures_file_readable = true;
        a
    }

    fn record_side(a: &mut App, t_ms: u32, down: bool) {
        library_event(a, &InEvent { type_: InType::PenButton, t_ms, side_button: down, ..Default::default() });
    }

    fn pen_card_tap(a: &mut App, id: i32, t_ms: u32) {
        let hit = a.lib.as_ref().unwrap().hits.h.iter().find(|h| h.id == id).copied().unwrap();
        let (x, y) = ((hit.r.x + hit.r.w / 2) as f32, (hit.r.y + hit.r.h / 2) as f32);
        library_event(a, &InEvent { type_: InType::PenDown, x, y, t_ms, ..Default::default() });
        library_event(a, &InEvent { type_: InType::PenUp, x, y, t_ms: t_ms + 40, ..Default::default() });
    }

    #[test]
    fn pen_record_timing_reassignment_buttons_and_save_failure() {
        let root = std::env::temp_dir().join(format!("grimoire-p6-library-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let mut a = pen_record_app();
        open_record_card(&mut a, SHORTCUT_UNDO);
        record_side(&mut a, 100, true);
        record_side(&mut a, 180, false);
        library_tick(&mut a, 529);
        assert_eq!(probe(&mut a, "record"), "wait");
        a.input_drained = false;
        library_tick(&mut a, 530);
        assert_eq!(probe(&mut a, "record"), "wait");
        a.input_drained = true;
        library_tick(&mut a, 530);
        assert_eq!(probe(&mut a, "record"), "pattern pen.Button.tap");
        pen_card_tap(&mut a, H_RECORD_RETRY, 600);
        assert_eq!(probe(&mut a, "record"), "wait");

        let key = GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Double };
        assert_eq!(gesturemap_set(&mut a.gestures, key, SHORTCUT_LINKS), 0);
        app_gestures_sync(&mut a);
        for t in [1000, 1200] { record_side(&mut a, t, true); record_side(&mut a, t + 80, false); }
        library_tick(&mut a, 1630);
        assert_eq!(probe(&mut a, "record"), "pattern pen.Button.double");
        assert_eq!(probe(&mut a, "record.line"), "Now for Links on this page. Use moves it here.");
        assert_eq!(gesturemap_action(&a.gestures, key), SHORTCUT_LINKS);
        // Failed persistence cannot move the live binding or close the candidate.
        lib(&mut a).gestures_file_readable = false;
        pen_card_tap(&mut a, H_RECORD_USE, 1700);
        assert_eq!(gesturemap_action(&a.gestures, key), SHORTCUT_LINKS);
        assert!(library_record_active(&a));
        lib(&mut a).gestures_file_readable = true;
        pen_card_tap(&mut a, H_RECORD_USE, 1800);
        assert!(!library_record_active(&a));
        assert_eq!(gesturemap_action(&a.gestures, key), SHORTCUT_UNDO);
        assert!(std::fs::read_to_string(gesture_path()).unwrap().contains("pen.Button.double=undo"));
        assert_eq!(a.toast.as_str(), "That gesture is no longer for Links on this page.");
        assert!(a.pb_config.waits[0]);

        // Clearing every multi-click binding updates timing without another file load.
        let mut next = a.gestures.clone();
        gesturemap_clear_action(&mut next, SHORTCUT_UNDO);
        gesturemap_clear_action(&mut next, SHORTCUT_CLEAR_PAGE);
        assert!(save_gesture_map(&mut a, next));
        assert!(!a.pb_config.waits[0]);
        open_record_card(&mut a, SHORTCUT_REDO);
        record_side(&mut a, 2000, true); record_side(&mut a, 2080, false);
        assert_eq!(probe(&mut a, "record"), "wait");
        library_tick(&mut a,2430);
        assert_eq!(probe(&mut a, "record"), "pattern pen.Button.tap");
        pen_card_tap(&mut a, H_RECORD_CANCEL, 2500);
        assert!(reset_gesture_map(&mut a));
        assert!(!a.pb_config.waits[0]);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn pen_record_hold_drag_restrictions_and_proximity() {
        let mut a = pen_record_app();
        open_record_card(&mut a, SHORTCUT_HOLD_ERASER);
        record_side(&mut a, 100, true); record_side(&mut a, 180, false);
        library_tick(&mut a, 530);
        assert!(lib(&mut a).record_key.is_none());
        assert_eq!(probe(&mut a, "record.line"), "Erase while held needs the button held: hold it or drag with it.");
        record_side(&mut a, 1000, true);
        library_tick(&mut a, 2199);
        assert!(lib(&mut a).record_key.is_none());
        a.input_drained = false;
        library_tick(&mut a, 2200);
        assert!(lib(&mut a).record_key.is_none());
        a.input_drained = true;
        library_tick(&mut a, 2200);
        assert_eq!(probe(&mut a, "record"), "pattern pen.Button.hold");
        record_side(&mut a, 2250, false);
        assert_eq!(probe(&mut a, "record"), "pattern pen.Button.hold");
        // Touch patterns and shape samples cannot enter the eraser's pen-only row.
        open_record_card(&mut a, SHORTCUT_HOLD_ERASER);
        record_pattern(&mut a, GestureKey::Touch { fingers: 2, kind: TouchKind::Tap });
        assert!(lib(&mut a).record_key.is_none());
        record_shape_sample(&mut a, FsSample::default());
        assert_eq!(lib(&mut a).record_sample_count, 0);
        assert_eq!(probe(&mut a, "record.line"), "Erase while held needs the button held: hold it or drag with it.");

        for action in [SHORTCUT_PEN_PAN, SHORTCUT_PEN_ZOOM] {
            open_record_card(&mut a, action);
            record_side(&mut a, 3000, true);
            library_tick(&mut a, 4200);
            assert_eq!(probe(&mut a, "record.line"), "Hold the pen button and drag with it.");
            assert!(lib(&mut a).record_key.is_none());
            record_side(&mut a, 4250, false);
            pen_card_tap(&mut a, H_RECORD_RETRY, 4300);
            record_side(&mut a, 4400, true);
            a.dirty = RECT_EMPTY;
            a.dirty_mode = RefreshMode::Fast;
            library_event(&mut a, &InEvent { type_: InType::PenDown, x: 800.0, y: 1100.0,
                side_button: true, t_ms: 4410, ..Default::default() });
            assert_eq!(probe(&mut a, "record"), "pattern pen.Button.drag");
            assert!(!lib(&mut a).record_pen_pointer);
            assert!(lib(&mut a).pressed.is_none());
            assert!(lib(&mut a).record_pen_redraw);
            assert!(rect_is_empty(a.dirty)); // No Ui/GL16 submission during contact.
            library_event(&mut a, &InEvent { type_: InType::PenUp, t_ms: 4450, ..Default::default() });
            assert!(!lib(&mut a).record_pen_redraw);
            assert!(!rect_is_empty(a.dirty));
            assert_eq!(a.dirty_mode, RefreshMode::Ui);
            record_side(&mut a, 4460, false);
            library_tick(&mut a, 5000);
            assert_eq!(probe(&mut a, "record"), "pattern pen.Button.drag");
        }
        open_record_card(&mut a, SHORTCUT_UNDO);
        record_side(&mut a, 6000, true); record_side(&mut a, 6080, false);
        library_event(&mut a, &InEvent { type_: InType::PenLeave, t_ms: 6100, ..Default::default() });
        library_tick(&mut a, 6500);
        assert_eq!(probe(&mut a, "record"), "wait");
        // A release queued just below the hold threshold wins over a later tick.
        open_record_card(&mut a, SHORTCUT_HOLD_ERASER);
        record_side(&mut a, 7000, true);
        a.input_drained = false;
        library_tick(&mut a, 8300);
        record_side(&mut a, 8199, false);
        a.input_drained = true;
        library_tick(&mut a, 8400);
        assert!(lib(&mut a).record_key.is_none());
        assert_eq!(probe(&mut a, "record"), "wait");
        library_free(&mut a);
        canvas_free(&mut a.screen);
    }

    #[test]
    fn gesture_rows_count_sources_once_and_hide_unsupported_slots() {
        use crate::fingershape_synth::{shape_frames, Draw};
        i18n_set("en");
        let mut a = App::default();
        gesturemap_defaults(&mut a.gestures);
        let touch3 = GestureKey::Touch { fingers: 3, kind: TouchKind::Tap };
        assert_eq!(gesturemap_set(&mut a.gestures, touch3, SHORTCUT_UNDO), 0);
        let tap = GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Tap };
        assert_eq!(gesturemap_set(&mut a.gestures, tap, SHORTCUT_UNDO), 0);
        let mut text = String::new();
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 2, &mut text);
        assert_eq!(text, "pen button click");
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 3, &mut text);
        assert_eq!(text, "3-finger tap +1");
        // More pen slots are still one source (including the legal built-in duplicate).
        gesture_binding_text_for_caps(&a, SHORTCUT_CLEAR_PAGE, 2, &mut text);
        assert_eq!(text, "pen button hold");
        let frames = shape_frames("Z", &Draw::default());
        let points: Vec<FsPt> = frames.iter().map(|f| FsPt { x: f.x, y: f.y }).collect();
        let mut sample = FsSample::default();
        fs_sample_of(&points, &mut sample);
        assert_eq!(gesturemap_shape_set(&mut a.gestures, SHORTCUT_UNDO, &[sample, sample]), 0);
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 2, &mut text);
        assert_eq!(text, "finger shape +1");
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 3, &mut text);
        assert_eq!(text, "3-finger tap +2");
        gesturemap_clear_action(&mut a.gestures, SHORTCUT_UNDO);
        gesturemap_pen_button2(&mut a.gestures, true);
        let second = GestureKey::Pen { button: PenBtn::Button2, kind: PenKind::Double };
        assert_eq!(gesturemap_set(&mut a.gestures, second, SHORTCUT_UNDO), 0);
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 2, &mut text);
        assert_eq!(text, "second pen button double-click");
        gesturemap_pen_button2(&mut a.gestures, false);
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 2, &mut text);
        assert_eq!(text, "no gesture");
        assert_eq!(gesturemap_parse(&mut a.gestures, b"pen.Eraser.tap=undo\n"), 0);
        gesture_binding_text_for_caps(&a, SHORTCUT_UNDO, 2, &mut text);
        assert_eq!(text, "no gesture");
        assert!(binding_row_index(GESTURES, 0, SHORTCUT_HOLD_ERASER).is_some());
        for action in [SHORTCUT_PEN_PAN, SHORTCUT_PEN_ZOOM] {
            assert!(binding_row_index(GESTURES, 2, action).is_some());
            assert!(binding_row_index(BUTTONS, 2, action).is_none());
        }
    }

    #[test]
    fn recording_contacts_buttons_timeout_and_rotation() {
        use crate::fingershape_synth::{shape_frames, Draw};
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        #[cfg(feature = "sim")]
        {
            // Line-only progress refreshes flush immediately, so provide the simulator's panel.
            let (mut w, mut h) = (a.w, a.h);
            crate::sim_sdl::display_set_orientation(0, &mut w, &mut h);
        }
        library_init(&mut a);
        gesturemap_defaults(&mut a.gestures);
        lib(&mut a).modal = Modal::Gestures;
        lib(&mut a).gestures_group = 0;
        open_record_card(&mut a, SHORTCUT_UNDO);
        assert_eq!(a.lib.as_ref().unwrap().hits.h.len(), 3);
        assert_eq!(probe(&mut a, "sheet"), "gestures");
        assert_eq!(probe(&mut a, "gestures_group"), "editing");
        assert_eq!(probe(&mut a, "gestures.learning"), "undo");
        let now = lib(&mut a).record_started;
        library_tick(&mut a, now.wrapping_add(LEARN_MS));
        assert_eq!(probe(&mut a, "record"), "0");

        open_record_card(&mut a, SHORTCUT_UNDO);
        library_event(&mut a, &InEvent { type_: InType::Gesture, value: -7, ..Default::default() });
        assert_eq!(probe(&mut a, "record.line"), "Two fingers spreading is kept for zoom.");
        let now = lib(&mut a).record_started;
        library_tick(&mut a, now.wrapping_add(LEARN_MS * 2));
        assert!(library_record_active(&a)); // A refused attempt also stops the initial timer.
        library_event(&mut a, &InEvent { type_: InType::Gesture, value: G_2_DOWN, ..Default::default() });
        assert_eq!(probe(&mut a, "record"), "pattern touch.2.down");
        let retry = a.lib.as_ref().unwrap().hits.h.iter().find(|h| h.id == H_RECORD_RETRY).copied().unwrap();
        let (x, y) = (retry.r.x + retry.r.w / 2, retry.r.y + retry.r.h / 2);
        assert_eq!(probe(&mut a, &format!("ui_at.{x}.{y}")), "record.retry");
        for type_ in [InType::TouchDown, InType::TouchUp] {
            library_event(&mut a, &InEvent { type_, x: x as f32, y: y as f32, t_ms: 200, ..Default::default() });
        }
        assert_eq!(probe(&mut a, "record"), "wait");

        let frames = shape_frames("V", &Draw::default());
        let points: Vec<FsPt> = frames.iter().map(|f| FsPt { x: f.x, y: f.y }).collect();
        let mut sample = FsSample::default();
        fs_sample_of(&points, &mut sample);
        record_shape_sample(&mut a, sample);
        assert_eq!(probe(&mut a, "record.line"), "Too easy to make by accident.\nGive it two turns or a loop.");
        assert_eq!(lib(&mut a).record_sample_count, 0);

        // Three actual shape captures run the existing sample, consistency and bank checks.
        for attempt in 0..3 {
            let frames = shape_frames("square", &Draw::default());
            for (i, f) in frames.iter().enumerate() {
                let previous = (lib(&mut a).record_drawn, lib(&mut a).record_last_frame);
                library_event(&mut a, &InEvent {
                    type_: if i == 0 { InType::TouchDown } else { InType::TouchMove },
                    x: f.x as f32, y: f.y as f32, t_ms: 1000 + attempt * 2000 + f.t,
                    ..Default::default()
                });
                if i > 0 && previous.0 != 0 && lib(&mut a).record_last_frame != previous.1 {
                    assert!(lib(&mut a).record_last_frame.wrapping_sub(previous.1) >= 40);
                }
            }
            let f = frames.last().unwrap();
            library_event(&mut a, &InEvent { type_: InType::TouchUp,
                x: f.x as f32, y: f.y as f32, t_ms: 1000 + attempt * 2000 + f.t,
                ..Default::default() });
            assert_eq!(lib(&mut a).record_sample_count, attempt as usize + 1);
            assert!(rect_is_empty(lib(&mut a).record_trail));
        }
        assert_eq!(probe(&mut a, "record"), "ready");
        assert!(lib(&mut a).record_checks_done);
        let samples = lib(&mut a).record_samples;
        library_leave(&mut a);
        a.relayout = true;
        library_enter(&mut a);
        assert_eq!(lib(&mut a).record_sample_count, 3);
        assert_eq!(lib(&mut a).record_samples[0].xy, samples[0].xy);

        // A contact beginning on Cancel becomes a drawing; its lift must not cancel.
        let cancel = a.lib.as_ref().unwrap().hits.h.iter().find(|h| h.id == H_RECORD_CANCEL).copied().unwrap();
        let (x, y) = (cancel.r.x + cancel.r.w / 2, cancel.r.y + cancel.r.h / 2);
        library_event(&mut a, &InEvent { type_: InType::TouchDown, x: x as f32, y: y as f32, t_ms: 9000, ..Default::default() });
        assert!(lib(&mut a).pressed.is_some());
        library_event(&mut a, &InEvent { type_: InType::TouchMove, x: (x - 100) as f32, y: y as f32, t_ms: 9100, ..Default::default() });
        library_tick(&mut a, 9180);
        assert!(lib(&mut a).pressed.is_none());
        library_event(&mut a, &InEvent { type_: InType::TouchUp, x: (x - 100) as f32, y: y as f32, t_ms: 9200, ..Default::default() });
        assert!(library_record_active(&a));

        // Single-finger swipes become patterns; a prompt second bar is a two-stroke refusal.
        open_record_card(&mut a, SHORTCUT_UNDO);
        for i in 0..=8 {
            library_event(&mut a, &InEvent { type_: if i == 0 { InType::TouchDown } else { InType::TouchMove },
                x: (700 + 50 * i) as f32, y: 1100.0, t_ms: 10000 + 40 * i,
                ..Default::default() });
        }
        library_event(&mut a, &InEvent { type_: InType::TouchUp, x: 1100.0, y: 1100.0, t_ms: 10330, ..Default::default() });
        assert_eq!(probe(&mut a, "record"), "pattern touch.1.right");
        library_event(&mut a, &InEvent { type_: InType::TouchDown, x: 700.0, y: 1200.0, t_ms: 10350, ..Default::default() });
        assert_eq!(probe(&mut a, "record.line"), "One stroke, without lifting.");
        library_event(&mut a, &InEvent { type_: InType::TouchUp, x: 700.0, y: 1200.0, t_ms: 10400, ..Default::default() });
        let cancel = a.lib.as_ref().unwrap().hits.h.iter().find(|h| h.id == H_RECORD_CANCEL).copied().unwrap();
        for type_ in [InType::TouchDown, InType::TouchUp] {
            library_event(&mut a, &InEvent { type_, x: (cancel.r.x + cancel.r.w / 2) as f32,
                y: (cancel.r.y + cancel.r.h / 2) as f32, t_ms: 11000, ..Default::default() });
        }
        assert_eq!(probe(&mut a, "record"), "0");

        assert_eq!(gesturemap_shape_set(&mut a.gestures, SHORTCUT_UNDO, &samples[..2]), 0);
        open_record_card(&mut a, SHORTCUT_REDO);
        {
            let s = lib(&mut a);
            s.record_samples = samples;
            s.record_sample_count = 2;
            s.record_phase = RecordPhase::Ready;
        }
        use_recording(&mut a);
        assert!(library_record_active(&a));
        assert!(lib(&mut a).record_checks_done);
        assert_eq!(lib(&mut a).record_same_action, SHORTCUT_UNDO);
        assert_eq!(probe(&mut a, "record.line"), "That is the shape for Undo. Use moves it here.");
        assert!(gesturemap_shape_of(&a.gestures, SHORTCUT_UNDO).is_some());
        assert!(gesturemap_shape_of(&a.gestures, SHORTCUT_REDO).is_none());
        close_record_card(&mut a);
        assert!(gestures_reset(&mut a));
        assert_eq!(a.lib.as_ref().unwrap().hits.h.len(), 2);
        assert!(lib(&mut a).gestures_reset_confirm);
        act(&mut a, &Hit { id: H_GESTURES_RESET_CANCEL, ..Default::default() });
        assert!(!lib(&mut a).gestures_reset_confirm);
        lib(&mut a).gestures_group = 2;
        act(&mut a, &Hit { id: H_GESTURE_ROW, arg: 2, ..Default::default() });
        assert_eq!(lib(&mut a).record_action, SHORTCUT_ZOOM_OUT);
        assert_eq!(lib(&mut a).record_next_action, SHORTCUT_ZOOM_IN);
        act(&mut a, &Hit { id: H_RECORD_RETRY, ..Default::default() });
        assert_eq!(lib(&mut a).record_next_action, SHORTCUT_ZOOM_IN);
        finish_recording(&mut a);
        assert_eq!(lib(&mut a).record_action, SHORTCUT_ZOOM_IN);
        finish_recording(&mut a);
        assert!(!library_record_active(&a));
        library_free(&mut a);
        canvas_free(&mut a.screen);
    }

    #[test]
    fn shape_thumbnail_stays_in_its_box_and_restores_clip() {
        ui_init(300.0);
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 150, 150), 0);
        c.px.fill(127);
        let old = Rect::new(3, 4, 140, 139);
        canvas_set_clip(&mut c, old);
        let r = Rect::new(20, 21, 96, 96);
        let mut g = FsGesture::default();
        let sample = FsSample { xy: std::array::from_fn(|i| if i % 3 == 0 { 255 } else { 0 }) };
        fs_prepare(&mut g, &[sample, sample]);
        draw_shape_thumb(&mut c, r, &g);
        assert_eq!(c.clip, old);
        let mut black = 0;
        for y in 0..c.h {
            for x in 0..c.w {
                let p = c.px[(y * c.stride + x) as usize];
                if rect_contains(r, x, y) {
                    assert!(p == 0 || p == 255);
                    black += (p == 0) as usize;
                } else { assert_eq!(p, 127); }
            }
        }
        assert!(black > 0);
        canvas_free(&mut c);
    }

    #[test]
    fn strtol10_reads_as_strtol() {
        assert_eq!(strtol10(b"3.luma"), (3, 1));
        assert_eq!(strtol10(b"12"), (12, 2));
        assert_eq!(strtol10(b" +7.name"), (7, 3));
        assert_eq!(strtol10(b"-2"), (-2, 2));
        assert_eq!(strtol10(b".luma"), (0, 0));
        assert_eq!(strtol10(b"x"), (0, 0));
        assert_eq!(
            strtol10(b"99999999999999999999999").0,
            libc::c_long::MAX as i64
        );
    }

    #[test]
    fn the_buttons_groups_rows() {
        assert_eq!(N_GROUPS, 7);
        assert_eq!(group_row_count(0), 10);
        assert_eq!(group_row_count(1), TOOL_PENS as usize + 2);
        // Zoom and scrolling are Pages rows; Index owns links and naming.
        assert_eq!(group_row_count(2), 11);
        assert_eq!(binding_row_count(BUTTONS, 2), 9);
        assert_eq!(binding_row_count(GESTURES, 2), 11);
        assert_eq!(learn_row_at(2, 2).unwrap().act, [SHORTCUT_ZOOM_OUT, SHORTCUT_ZOOM_IN]);
        assert_eq!(learn_row_at(2, 7).unwrap().act, [SHORTCUT_LANDSCAPE_FIT, 0]);
        assert_eq!(learn_row_at(2, 8).unwrap().act, [SHORTCUT_GESTURE_NEW_PAGE, 0]);
        assert_eq!(group_row_count(3), 8);
        assert_eq!(ACTION_GROUPS[3].probe, "index");
        assert_eq!(learn_row_at(3, 7).unwrap().act, [SHORTCUT_LINK_AREAS, 0]);
        assert_eq!(group_row_count(4), 9);
        assert!(learn_row_at(4, 9).is_none());
        assert_eq!(group_row_count(5), 10);
        assert_eq!(group_row_count(6), 6);
        assert_eq!(binding_row_count(BUTTONS, 6), 4);
        assert_eq!(binding_row_count(GESTURES, 0), 10); // Hold eraser is pen-only here.
        assert_eq!(binding_row_at(BUTTONS, 6, 1).unwrap().act, [SHORTCUT_LANGUAGE, 0]);
        assert_eq!(learn_row_at(1, 0).unwrap().act, [SHORTCUT_TOOL_BASE + TOOL_ORDER[0].0, 0]);
        assert_eq!(learn_row_at(1, TOOL_PENS as usize).unwrap().act, [SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL]);
        for g in 0..N_GROUPS {
            for i in 0..group_row_count(g) {
                let r = learn_row_at(g, i).unwrap();
                for &act in r.act.iter().filter(|&&x| x != 0) {
                    assert_ne!(action_label(act), "?", "group {g} row {i}");
                }
            }
        }
    }

    #[test]
    fn gesture_sheets_fit_in_both_languages_and_orientations() {
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        font_set_overflow_hook(Some(|text, need, have| panic!("{text:?} needs {need}px, has {have}px")));
        for language in ["en", "es"] {
            i18n_set(language);
            for theme in [UiTheme::Gothic, UiTheme::Classic] {
                ui_set_theme(theme);
                for (w, h) in [(1860, 2480), (2480, 1860)] {
                    let mut a = App { w, h, dpi: 300.0, ..Default::default() };
                    assert_eq!(canvas_init(&mut a.screen, w, h), 0);
                    library_init(&mut a);
                    gesturemap_defaults(&mut a.gestures);
                    lib(&mut a).modal = Modal::Gestures;
                    for group in -1..N_GROUPS as i32 {
                        lib(&mut a).gestures_group = group;
                        hits_clear(&mut lib(&mut a).hits);
                        draw_bind_sheet(&mut a, GESTURES);
                    }
                    library_free(&mut a);
                    canvas_free(&mut a.screen);
                }
            }
        }
        font_set_overflow_hook(None);
        i18n_set("en");
        ui_set_theme(UiTheme::Gothic);
    }

    #[test]
    fn buttons_rows_have_separate_reachable_touch_targets() {
        assert_eq!(font_init(), 0);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            for (w, h, dpi) in [(1860, 2480, 300.0), (1986, 2648, 320.0)] {
                ui_init(dpi);
                for (w, h) in [(w, h), (h, w)] {
                    let mut a = App {
                        w,
                        h,
                        dpi,
                        ..Default::default()
                    };
                    assert_eq!(canvas_init(&mut a.screen, w, h), 0);
                    library_init(&mut a);
                    for kind in [BUTTONS, GESTURES] {
                        for group in 0..N_GROUPS {
                            lib(&mut a).modal = kind.modal;
                            bind_group_set(kind, lib(&mut a), group as i32);
                            hits_clear(&mut lib(&mut a).hits);
                            draw_modal(&mut a);
                            let hits = &a.lib.as_ref().unwrap().hits;
                            assert_eq!(hits.h.len(), binding_row_count(kind, group) + 1);
                            let mut bottom = 0;
                            for (row, hit) in hits.h.iter().enumerate() {
                                let r = hit.r;
                                assert!(r.y > bottom && r.y + r.h < h - ui_px(40.0));
                                assert!(r.x >= 0 && r.x + r.w <= w);
                                assert!(r.h >= (dpi * 10.0 / 25.4).floor() as i32);
                                let found =
                                    hits_find(hits, (r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32)
                                        .unwrap();
                                assert_eq!(
                                    (found.id, found.arg),
                                    if row < binding_row_count(kind, group) {
                                        (bind_row_hit(kind), row as i32)
                                    } else {
                                        (bind_done_hit(kind), 0)
                                    }
                                );
                                bottom = r.y + r.h;
                            }
                        }
                        bind_group_set(kind, lib(&mut a), -1);
                        hits_clear(&mut lib(&mut a).hits);
                        draw_modal(&mut a);
                        let mut bottom = 0;
                        for hit in &a.lib.as_ref().unwrap().hits.h {
                            assert!(hit.r.y > bottom && hit.r.y + hit.r.h < h - ui_px(40.0));
                            assert!(hit.r.x >= 0 && hit.r.x + hit.r.w <= w);
                            bottom = hit.r.y + hit.r.h;
                        }
                        if kind.modal == Modal::Gestures {
                            assert!(probe(&mut a, "gestures.1").starts_with("Gestures:"));
                        }
                    }
                    library_free(&mut a);
                    canvas_free(&mut a.screen);
                }
            }
        }
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
    }
}

/// Resolve a runner tap through the same hit names used by ui_at probes.
pub fn library_control_point(a:&mut App,name:&str)->Option<(f32,f32)> {
    if name.starts_with("notebook.")||name.starts_with("folder.") {
        let s=a.lib.as_deref()?;
        let i=s.items.iter().position(|it|if let Some(id)=name.strip_prefix("notebook."){!it.is_folder&&s.books[it.nb].id==id}else{it.is_folder&&it.folder==name.strip_prefix("folder.").unwrap()})?;
        return s.hits.h.iter().find(|h|h.id==H_CARD&&h.arg==i as i32).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32));
    }
    let points:Vec<_>=a.lib.as_deref()?.hits.h.iter().map(|h|((h.r.x+h.r.w/2) as f32,(h.r.y+h.r.h/2) as f32)).collect();
    for (x,y) in points {
        let mut out=String::new();
        if library_probe(a,&format!("ui_at.{}.{}",x as i32,y as i32),&mut out,128)==0 && out==name{return Some((x,y));}
    }None
}

pub fn library_settings_from_notebook(a:&mut App) {
    let fit=a.settings.landscape_fit;let s=lib(a);s.pressed=None;s.settings_return_editor=true;s.settings_editor_fit=fit;
    open_modal(a,Modal::Settings);
}

/// The notebook hosts a settings dialog and all its nested sheets without changing screens.
pub fn library_settings_active(a:&App)->bool {
    a.scr==ScreenId::Editor && a.lib.as_deref().is_some_and(|s|s.settings_return_editor && !s.settings_painting)
}
pub fn library_status_preview(a:&App)->bool {
    a.scr==ScreenId::Editor && a.lib.as_deref().is_some_and(|s|s.settings_return_editor && s.modal==Modal::Display)
}
pub fn library_settings_redraw(a:&mut App) {redraw(a,RefreshMode::Ui);}
pub fn library_settings_back(a:&mut App) {close_modal(a);}
