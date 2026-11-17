//! library.c: the home screen. Folders and notebooks as cards, paged (no scrolling on e-ink),
//! plus the sheets: new notebook (paper picker), settings, notebook actions, move, delete.

use crate::prelude::*;
use crate::about::AboutView;
use crate::settingsmenu;
use crate::ui::{ui_row, ui_row_page, ui_row_pager, Row as UiRow};
use crate::browser::{BrowseMode, Browser, BrowserResult, FilesPrefs, PickSpec};
use crate::fsview::StoragePath;
use crate::home::{home_folder_there, Home};

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
    About = 18,     // (Rust only: the bat's What's new)
    /// Esc at the top level, "Exit Grimoire?" (Rust only; numbered from 300, past the C's sheets: the probe
    /// names it apart from SHEETS)
    Exit = 300,
    /// Settings > Pair (2.0.0-beta.2, Rust only; a number of its own past the C's, as Exit's): the Bluetooth
    /// helper on or off, Pair a controller... (the Pair sheet above), and what is connected
    PairMenu = 330,
    /// Settings > Font (2.1, Rust only, Classic only; a number of its own as PairMenu's): the families
    /// Classic's text can be drawn in, with a sample, applied on Done
    Font = 340,
    /// A folder moved or renamed onto the name of one that is there (2.1, Rust only; a number of its own as
    /// Font's): Merge into it, Keep both (as "Name 2"), Cancel
    Merge = 350,
    /// Settings > Wi-Fi (2.1.0-beta.2, Rust only; a number of its own as Merge's): the radio, the network
    /// connected to and the Kindle's saved networks, a tap switching to one (wifi.rs)
    Wifi = 360,
    /// Settings > Pen > the pen button (Rust only; a number of its own as Wifi's): what holding the
    /// side button while writing does, as Language's list of choices
    SideButton = 370,
    /// Settings > Pen > the eraser (Rust only): whole strokes, or precise
    EraserMode = 380,
    /// Settings > System > About (Rust only): what this Kindle is, the storage used, and its links
    DeviceInfo = 390,
    /// Settings > System > About > a link (Rust only): the link as a QR code a phone scans
    LinkQr = 395,
    /// The Files card's menu (2.2, Rust only; a number of its own as LinkQr's): Open, Hide from Library, Cancel
    FilesMore = 397,
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
    WifiPassword = 5,  // the Wi-Fi sheet's masked password entry
    WifiName = 6,      // the Wi-Fi sheet's hidden network name entry
    WifiEapUser = 7,   // the Wi-Fi sheet's enterprise username entry
    WifiEapPassword = 8, // the Wi-Fi sheet's masked enterprise password entry
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
const H_SIDE_PICK: i32 = 120;
const H_SIDE_BACK: i32 = 121;
const H_SET_ERASER: i32 = 122;
const H_ERASER_PICK: i32 = 123;
const H_ERASER_BACK: i32 = 124;
const H_SET_DEVICE: i32 = 125;
const H_DEVICE_BACK: i32 = 126;
const H_LINK: i32 = 127;
const H_LINK_PAGE: i32 = 128;
const H_LINK_BACK: i32 = 129;
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
// 2.1: the record card's Disable, which saves "nothing" for its action (over the built-ins too).
const H_RECORD_DISABLE: i32 = 98;
const H_GESTURES_RESET_CONFIRM: i32 = 67;
const H_GESTURES_RESET_CANCEL: i32 = 68;
const H_ABOUT: i32 = 71;
const H_ABOUT_CLOSE: i32 = 72;
// (73 was Don't show this again, gone in 2.0.0: the owner's rule of 2026-11-05)
const H_ABOUT_PREV: i32 = 74;
const H_ABOUT_NEXT: i32 = 75;
const H_ABOUT_MORE: i32 = 76;
// 2.1.0: Read all's list of every version's notes, its rows (arg: the entry in about::notes()), and Back
// from a version's notes to it.
const H_ABOUT_ROW: i32 = 87;
const H_ABOUT_BACK: i32 = 88;
const H_FMORE_MOVE: i32 = 77;
// 2.0.0-beta.2: a notebook's or a folder's Leave out of the Graph / Show in the Graph.
const H_MORE_GRAPH: i32 = 78;
const H_FMORE_GRAPH: i32 = 79;
// 2.0.0-beta.2: the Exit Grimoire? dialog's Yes and No.
const H_EXIT_YES: i32 = 82;
const H_EXIT_NO: i32 = 83;
// 2.0.0-beta.2, the owner's ruling of 2026-11-04 (Settings' Controller split into Shortcuts and Pair): Settings'
// Pair button, and the Shortcuts sheet's device and its own shortcuts on or off.
const H_PAIR_MENU: i32 = 84;
const H_SHORTCUT_DEVICE: i32 = 85;
const H_SHORTCUT_OWN: i32 = 86;
// 2.1: the Font row (Classic only; a chip on the Style row before #115), and the Font sheet's rows (arg: the
// family), Done (the page's X since #115) and Cancel (its Back).
const H_SET_FONT: i32 = 89;
const H_FONT_PICK: i32 = 90;
const H_FONT_DONE: i32 = 91;
const H_FONT_CANCEL: i32 = 92;
// 2.1: the Merge dialog's Merge into it and Keep both (Cancel is H_CANCEL).
const H_MERGE_INTO: i32 = 93;
const H_MERGE_BOTH: i32 = 94;
// 2.1.0-beta.2: New notebook's From a file... (the file picker over it).
const H_NEW_FILE: i32 = 95;
// 2.1.0-beta.2: the Font sheet's rows of the owner's own fonts (arg: the font, an index of LibraryState.fonts),
// and the pages of them (arg: -1 back, 1 on).
const H_FONT_OWN: i32 = 96;
const H_FONT_PAGE: i32 = 97;
// The page arrows of Settings' lists (ui_row_page): the categories down the left, the rows of the open
// category, and a submenu's rows (arg: -1 back, 1 on).
const H_SET_CAT_PAGE: i32 = 130;
const H_SET_ROWS_PAGE: i32 = 131;
const H_SETTINGS_PAGE: i32 = 132;
// 2.1.0-beta.2: Settings' Wi-Fi button, and its submenu's rows: the radio (arg: WIFI_*), a saved network
// (arg: its row, an index of WifiSheet.rows). (Numbered apart from the ids other work of the same release
// adds after 97.)
const H_WIFI_MENU: i32 = 180;
const H_WIFI_RADIO: i32 = 181;
const H_WIFI_NET: i32 = 182;
// 2.1: Settings' Sleep & Clock buttons (the Display sheet's sleep and clock rows, on pages of their own).
const H_SET_SLEEP: i32 = 184;
const H_SET_CLOCK: i32 = 185;
// Settings > Display's Dark mode toggle (the invert setting, moved out of the Display sheet).
const H_SET_DARK: i32 = 196;
// The Files card's menu (2.2): Open (Files, place mode) and Hide from Library.
const H_FILES_OPEN: i32 = 197;
const H_FILES_HIDE: i32 = 198;
// A folder's menu's home row (2.2, issue #160): Set as home, or on the folder that is home, Reset home to Notebooks
// (arg: 1 for the reset). Numbered apart from the ids above, which other work adds after.
const H_FMORE_HOME: i32 = 210;
// Settings > Wi-Fi's top controls and lists: Nearby/Saved (arg: 0 or 1), Rescan, Other... (a hidden
// network), a saved row's trash (arg: a row), and a network a scan found (arg: an index of
// WifiSheet.near). The saved rows keep H_WIFI_NET.
const H_WIFI_TAB: i32 = 186;
const H_WIFI_SCAN: i32 = 187;
const H_WIFI_NEAR: i32 = 188;
const H_WIFI_DELETE: i32 = 189;
const H_WIFI_OTHER: i32 = 190;
const H_WIFI_OTHER_OPEN: i32 = 191;
const H_WIFI_OTHER_WPA: i32 = 192;
const H_WIFI_OTHER_CANCEL: i32 = 193;
const H_WIFI_OTHER_EAP: i32 = 194;
const H_WIFI_OTHER_WPA3: i32 = 195;

// 2.1: Settings' page (a full screen, the categories down its left): the category a tap picks (arg: its
// index), whose settings the page shows.
const H_SET_CAT: i32 = 100;
// A Settings submenu's Back and Done: one level up (Settings, Shortcuts or Pair), never out of Settings.
const H_SETTINGS_BACK: i32 = 101;
/// Every Settings page's X: the whole of Settings closes (a submenu's Back stays one step back).
const H_SETTINGS_CLOSE: i32 = 102;

#[derive(Clone, Copy, Default)]
struct Item {
    is_folder: bool,
    /// the Files card (2.2): the top level's last card while settings.ini files_card says so (no notebook, no
    /// folder: `nb` and `folder` mean nothing)
    is_files: bool,
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
    /// Exit Grimoire?'s highlight is on No (Left and Right move it; Enter presses it)
    exit_no: bool,
    /// The Font sheet (2.1): the family in use (FONT_FAMILIES), and where its sample card was drawn.
    font_pick: usize,
    /// The Font sheet's own fonts (2.1.0-beta.2): the files of grimoire/fonts as the sheet found them
    /// (userfonts_scan), the one highlighted (its path under the folder; "" while a family is: font_pick), the
    /// first shown of the page on show and how many a page holds, the rows drawn (each font's index and where),
    /// how the sample was drawn (the runner's probe), and when the cache was last looked at.
    fonts: Vec<crate::userfonts::UserFont>,
    font_file: String,
    font_first: usize,
    font_per_page: usize,
    font_own_rows: Vec<(usize, Rect)>,
    /// a family was tapped on the sheet since it opened (an apply then drops a font=file: that does not draw)
    font_picked: bool,
    font_sample_how: &'static str,
    fonts_looked_ms: u32,
    /// The file picker (2.1.0-beta.2, browser.rs), open over New notebook's sheet (which stays under it: Cancel
    /// and Esc at the picker's root come back to it), and whether the sheet's name was typed (D28: a typed name is
    /// the imported notebook's, else the file's own).
    files: Browser,
    /// The Files card's second line (2.2), the storage's free space: read when the Library reloads (one statvfs),
    /// not at each draw of the card; None when there is no storage to tell
    files_free: Option<String>,
    new_name_typed: bool,
    /// The Merge dialog (2.1): the folder that was moved or renamed, and the path it would take, which is
    /// the name of a folder already there
    merge_old: FixedStr<STORE_NAME_LEN>,
    merge_dest: FixedStr<STORE_NAME_LEN>,
    settings_return_editor: bool,
    settings_painting: bool,
    settings_editor_fit: bool,
    /// Settings' page: the category whose settings show (0 General, 1 Display, 2 Sleep & Clock,
    /// 3 Pen, 4 Controls, 5 System; H_SET_CAT), and the page each of its lists is on (ui_row_page):
    /// the categories, the open category's rows, and the submenu's rows (whose title keys the last,
    /// so another submenu starts at its first page).
    settings_cat: i32,
    settings_cat_page: i32,
    /// The category the category page was last laid out for: opening Settings or picking one brings
    /// its page on screen, while a tap on the page arrows stays on the page it asked for.
    settings_cat_shown: i32,
    settings_rows_page: i32,
    settings_menu_page: i32,
    settings_menu_title: String,
    /// About's links: the page its list is on, and the link whose QR code shows (its index in
    /// ABOUT_LINKS).
    device_links_page: i32,
    device_link: i32,
    /// A screen Settings opened (Pen feel, Calibration) is returning to the Library: library_enter
    /// opens Settings again, as it was left.
    reopen_settings: bool,
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
    /// App.hid_changes as the Shortcuts sheet's devices, the Pair sheet's status line or Buttons' rows last
    /// showed
    shown_hid_changes: u32,
    /// The Shortcuts sheet's device (its profile's slug, shortcut_profile_slug; "" none yet): whose own
    /// shortcuts its row turns on or off.
    shortcut_dev: String,
    /// The device whose own buttons the Buttons sheet shows and learns into, fixed as it opens (buttons_dev_for;
    /// "" for the default, controller.ini).
    buttons_dev: String,
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
    /// The About sheet (What's new): its view (the card, Read all's list, a version's notes), the
    /// version's notes shown (their entry in about::notes()), the list's page and the notes' page, and
    /// where the bat is (the card hangs from it). As it last drew: the view's pages, the list's
    /// entries on its page and the row marked as this version's, whether the signature shows, the
    /// card's notes (the blocks shown, of how many, the last one cut), a version's notes' body size (0
    /// elsewhere), and where the card or sheet is (a step on the sheet refreshes there alone:
    /// about_step).
    about_view: AboutView,
    about_note: usize,
    about_list_page: i32,
    about_page: i32,
    about_pages: i32,
    about_bat: Rect,
    about_rows: Vec<usize>,
    about_marked: i32,
    about_signed: bool,
    about_card: (i32, i32, bool),
    about_body_px: f32,
    about_lead: f32,
    about_area: Rect,
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
    // The Pair sheet (Settings > Pair > Pair a controller): what it asked the Bluetooth helper
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
    /// Settings > Wi-Fi
    wifi: WifiSheet,
    /// The Wi-Fi submenu was opened from Share's bottom button: Back or X returns to Share.
    share_wifi_return: bool,
    // The Updates sheet: its check (UPD_*), the newest version it found, why it failed.
    upd: i32,
    /// at most UPDATE_VERSION_LEN - 1 bytes
    upd_latest: String,
    /// at most 15 bytes
    upd_why: String,
    modal: Modal,
    kbp: KbPurpose,
    kb: Keyboard,
    /// the sheet the keyboard was opened over: it stays drawn under its keys (Settings' and the
    /// Wi-Fi sheet's keyboards never close their page)
    kb_parent: Modal,
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
    /// when the finger went down on it (a Buttons row held HOLD_MS: forget_row)
    pressed_at: u32,
    /// The Graph as a place of its own (2.0.0-beta.2, the owner's ruling of 2026-11-03): opened from the
    /// Library (two fingers down, the index-map key) on every notebook, with no notebook
    /// open. Its notebooks' browser (past the caps); what it keeps while a notebook it opened is in front; and whether
    /// the Library comes back to it as it enters (Back from that notebook).
    map: MapView,
    map_picker: NotebookPicker,
    map_keep: Option<MapKeep>,
    map_return: bool,
    /// Home (2.2, issue #160; home.rs). The main menu has not been used yet in this run: while that is so it
    /// opens at home as it comes in front (library_enter), and the first contact, gesture or action on it ends
    /// that. A home inside Files that waited as Grimoire started: for What's new (open over the Library at the
    /// first start after an update; its closing goes home, close_modal), and once for the notebook KOReader
    /// started Grimoire on (home_waits).
    home_pending: bool,
    home_deferred: bool,
    home_held: bool,
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
/// The top bar's buttons' height, centred in the bar (draw_base).
fn top_btn_h() -> i32 {
    ui_px(150.0)
}
/// Where the top bar's buttons end; about.rs starts a sheet that would cut across them at the margin.
pub(crate) fn top_buttons_foot() -> i32 {
    (top_h() - top_btn_h()) / 2 + top_btn_h()
}
/// The pager's bar at the foot (its rule 10 px above it); about.rs keeps a version's notes off it.
pub(crate) fn bottom_h() -> i32 {
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
    let files_card = a.settings.files_card;
    let s = lib(a);
    free_lists(s);
    s.card_gen = bgimage_generation(); // (the cards read from now on are up to date)
    store_list(&mut s.books);
    store_list_folders_from(&s.books, &mut s.folders); // (each notebook.ini read once)
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
    // Where the Files card goes (2.2), decided here and nowhere else: after every folder and notebook of the top
    // level, so it is the Library's last card (on its last screen), and no notebook's place changes for it. (The
    // last card of the first screen instead would be an insert at the screen's last place here.)
    if files_card && s.cur_folder.is_empty() {
        s.items.push(Item { is_files: true, ..Default::default() });
        s.files_free = crate::browser::files_free_line();
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
    if it.is_files {
        draw_files_card(c, s, r, idx);
        return;
    }
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

/// The Files card (2.2): Files' icon where a notebook's page is, its name, the storage's free space as Files' root
/// says it (left out when it does not fit), and its `...` (Open, Hide from Library).
fn draw_files_card(c: &mut Canvas, s: &mut LibraryState, r: Rect, idx: usize) {
    let th = r.w * 4 / 3;
    let tb = Rect::new(r.x, r.y, r.w, th);
    ui_frame(c, tb, ui_px(16.0), 255, true, 0);
    ui_icon(c, IconKind::Files, rect_inflate(tb, -r.w / 5), 0);
    ui_frame(c, tb, ui_px(16.0), 0, false, ui_px(4.0));
    let more = ui_px(110.0);
    let tr_ = Rect::new(r.x, r.y + th + ui_px(14.0), r.w - more - ui_px(10.0), ui_px(96.0));
    let face = if ui_theme() == UiTheme::Gothic { FontFace::Regular } else { FontFace::Bold };
    ui_text_name(c, face, ui_text_size() * 0.88, ui_small_size(), tr_, tr("Files"), 2, 0);
    if let Some(line) = s.files_free.as_deref() {
        let lr = Rect::new(r.x, tr_.y + tr_.h, r.w - more - ui_px(10.0), ui_px(40.0));
        let ls = ui_small_size();
        let fs = if font_measure(FontFace::Regular, ls, &line) > lr.w { font_fit_px(FontFace::Regular, ls, ls * 0.8, &line, lr.w) } else { ls };
        if fs > 0.0 {
            font_draw_left(c, FontFace::Regular, fs, lr, &line, 60);
        }
    }
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
    let bh = top_btn_h();
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
    const NBTNS: usize = 6;
    // A word too long for its button (a translation) widens all five alike, as far as leaves the
    // app's name room in the bar (beyond that a word is cut, and the fit check hears of it).
    for b in btns.iter() {
        let need = ui_button_need_w(
            bh,
            b.icon,
            b.label,
            if b.id == H_NEW {
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
    for b in btns.iter().take(NBTNS) {
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
            if b.id == H_NEW {
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
    } else {
        // The bat, Grimoire's mark: a tap on it (or on the name beside it) opens What's new.
        let k = ui_px(3.0);
        let bat = Rect::new(x, (th - BAT_H * k) / 2, BAT_W * k, BAT_H * k);
        draw_bat(c, bat.x, bat.y, k);
        s.about_bat = bat;
        x += bat.w + ui_px(34.0);
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
        let name_w = font_measure(FontFace::Title, title_px, title).min(title_rect.w);
        hits_add(&mut s.hits, Rect::new(0, 0, x + name_w + ui_px(20.0), th), H_ABOUT, 0);
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

/// Where a sheet `h` px tall goes on a screen (Files' Delete question is drawn in the same place: browser.rs).
pub(crate) fn sheet_rect(aw: i32, ah: i32, mut h: i32) -> Rect {
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
    let g = paper_grid_cols(
        PAPER_SHEET_TILES,
        in_.w,
        in_.h - ui_px(80.0) - ui_px(420.0),
        gap,
        ui_px(100.0),
        ui_px(300.0),
        PAPER_PICKER_COLS,
    );
    let gridw = g.cols * g.tw + (g.cols - 1) * gap;
    let ty = in_.y + ui_px(80.0);
    let tpl = !s.new_tpl.is_empty();
    let new_custom = !tpl && paper_is_custom(s.new_paper);
    for i in 0..PAPER_SHEET_TILES {
        // The last tile, Custom music: the chosen layout, or the last one built. (More layouts...,
        // the Templates sheet, is a button beside Cancel: S->new_paper is then what the notebook is
        // made on if the template cannot be used.)
        let custom = i == PICKER_PAPERS;
        let k = if custom {
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
        if let Some(pv) = paper_preview(s, dpi, k) {
            canvas_blit_fit(c, pv, tr_);
        }
        let sel = if custom { new_custom } else { !tpl && s.new_paper == k };
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
            // TRANSLATORS: the last paper's tile, here and in the editor's paper sheet: it opens a sheet
            // that builds a music paper from parts (192 px at 31 px in New notebook sideways, on one
            // line or two).
            label = tr("Custom music...");
        }
        // (A label wider than its tile goes on two lines under it, as most do sideways: labels side
        // by side never run into each other.)
        paper_tile_label(c, tr_, gap, label, sel);
        hits_add(
            &mut s.hits,
            Rect::new(tr_.x, tr_.y, tr_.w, g.th + ui_px(90.0)),
            if custom { H_PAPER_CUSTOM } else { H_PAPER },
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
    // More layouts... (the Templates sheet) beside Cancel, selected while a template is chosen.
    let more = Rect::new(cancel.x + bw + ui_px(30.0), cancel.y, bw, bh);
    ui_button(c, more, IconKind::None, tr("More layouts..."), if tpl { ButtonStyle::Selected } else { ButtonStyle::Plain });
    hits_add(&mut s.hits, more, H_PAPER_MORE, PAPER_BLANK.0 as i32);
    // From a file... (the file picker) beside it: a PDF or a folder of pictures becomes a notebook.
    let file = Rect::new(more.x + bw + ui_px(30.0), cancel.y, bw, bh);
    // TRANSLATORS: New notebook's button (348 px at 44 px, or 34 px, or two lines of that): it opens the file picker
    // to make a notebook of a PDF or a folder of pictures anywhere on the Kindle.
    ui_button(c, file, IconKind::None, tr("From a file..."), ButtonStyle::Plain);
    hits_add(&mut s.hits, file, H_NEW_FILE, 0);
}

/// A row of draw_menu's list (the C's MenuRow, its label owned).
struct MenuRow {
    icon: IconKind,
    label: String,
    id: i32,
    arg: i32,
    st: ButtonStyle,
    /// the label holds a name from a device (the Shortcuts sheet's device row): drawn smaller, or on two lines,
    /// then shortened as names are (ui_text_name), never counted as cut short
    name: bool,
}

// A notebook's menu row for the Graph of all notebooks (2.0.0-beta.2, the owner's ruling of 2026-11-03):
// Leave out of the Graph, or Show in the Graph when it is left out; greyed out, saying so, when a folder it
// is in is left out (that folder's own row shows it again).
fn graph_omit_row(nb: &NotebookInfo, id: i32) -> MenuRow {
    let folders = store_graph_omit_folders();
    if store_folder_graph_omitted(&folders, nb.folder.as_str()) {
        return menu_row(IconKind::Grid, graph_omit_with_folder(), id, 0, ButtonStyle::Disabled);
    }
    graph_toggle_row(nb.graph_omit, id)
}
// The same for a folder: it covers everything in it, folders inside it and notebooks moved in later included.
fn graph_omit_folder_row(path: &str) -> MenuRow {
    let folders = store_graph_omit_folders();
    let own = folders.iter().any(|f| f == path);
    if !own && store_folder_graph_omitted(&folders, path) {
        return menu_row(IconKind::Grid, graph_omit_with_folder(), H_FMORE_GRAPH, 0, ButtonStyle::Disabled);
    }
    graph_toggle_row(own, H_FMORE_GRAPH)
}
fn graph_toggle_row(omitted: bool, id: i32) -> MenuRow {
    // TRANSLATORS: a notebook's and a folder's menu (rows of 1470 px at 44 px in bold): its pages are not shown
    // in the Graph of all notebooks (the Graph: the Index's map of linked pages), or are shown there again.
    let label = if omitted { tr("Show in the Graph") } else { tr("Leave out of the Graph") };
    menu_row(IconKind::Grid, label, id, if omitted { 0 } else { 1 }, ButtonStyle::Plain)
}
fn graph_omit_with_folder() -> &'static str {
    // TRANSLATORS: a notebook's or a folder's menu row, greyed out (1470 px at 44 px in bold): a folder it is in is
    // left out of the Graph, which leaves it out too; that folder's menu shows it again.
    tr("Left out of the Graph with its folder")
}

fn menu_row(icon: IconKind, label: &str, id: i32, arg: i32, st: ButtonStyle) -> MenuRow {
    MenuRow {
        icon,
        label: label.to_string(),
        id,
        arg,
        st,
        name: false,
    }
}

// ---- the controller's buttons (Settings > Shortcuts > Buttons) ----

const LEARN_MS: u32 = 10000;

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

// "South, Up" for an action's buttons (those the Buttons sheet shows, buttons_map), "none" when it has none;
// "B, Up" while the Shortcuts sheet's device, or else the first controller connected, is a Switch-mode one,
// as its buttons are printed.
fn pads_text(a: &App, action: i32, out: &mut String, cap: usize) {
    let mut codes = [0i32; GROUP_ROWS_MAX];
    let n = shortcuts_pads_for(buttons_map(a), ActionSrc::Pad, action, &mut codes);
    let mut h = [InputHid::default(); 4];
    let nh = input_hids(&mut h);
    let chosen = a.lib.as_deref().map_or("", |s| s.shortcut_dev.as_str());
    let mut switch_layout = None;
    for hid in h.iter().take(nh.max(0) as usize) {
        if !hid.gamepad {
            continue;
        }
        if shortcut_profile_slug(hid.name.as_str(), hid.vendor, hid.product) == chosen {
            switch_layout = Some(hid.switch_layout);
            break;
        }
        switch_layout.get_or_insert(hid.switch_layout);
    }
    let switch_layout = switch_layout.unwrap_or(false);
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
    let cp = buttons_pads_path(a);
    if shortcuts_forget(
        buttons_map_mut(a),
        &cp,
        ActionSrc::Pad,
        row.act[0],
        row.act[1],
    ) != 0
    {
        buttons_save_failed(a);
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
    let cp = buttons_pads_path(a);
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
        app_toast(a, tr("Couldn't save gestures."), 3000);
        return false;
    }
    if gesturemap_save(&next, &gesture_path()) < 0 {
        app_toast(a, tr("Couldn't save gestures."), 3000);
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
        app_toast(a, tr("Couldn't save gestures."), 3000);
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
        csnprintf!(&mut msg, 200, ng_("Couldn't use %d line of gestures.ini.", "Couldn't use %d lines of gestures.ini.", n as i64), n);
        app_toast(a, &msg, 3500);
    } else if n < 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px bold or two lines): the current gestures.ini
        // is unreadable; Reset gestures is the recovery action.
        app_toast(a, tr("Couldn't read gestures.ini: Reset makes a new one."), 4000);
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

// Disable saves "nothing" for the card's action, which also overrides a built-in default
// (gesturemap_clear_action marks the slots set). A paired row disables both its actions.
fn disable_recording(a: &mut App) {
    let (action, paired) = {
        let s = lib(a);
        (s.record_action, s.record_next_action)
    };
    let mut next = a.gestures.clone();
    gesturemap_clear_action(&mut next, action);
    if paired != SHORTCUT_NONE {
        gesturemap_clear_action(&mut next, paired);
    }
    if !save_gesture_map(a, next) {
        return;
    }
    // (a pair that lost both its actions is named as its row is, "Next and previous page", as a row held says)
    let group = lib(a).gestures_group.max(0) as usize;
    let label = binding_row_index(GESTURES, group, action)
        .and_then(|i| binding_row_at(GESTURES, group, i))
        .and_then(|row| row.label)
        .filter(|_| paired != SHORTCUT_NONE)
        .map(tr)
        .unwrap_or_else(|| action_label(action));
    let mut msg = String::new();
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Disable in the record
    // card removed the action's gestures; %s is the action's name ("Undo", "Next page").
    csnprintf!(&mut msg, 200, tr("No gestures for %s now."), label);
    app_toast(a, &msg, 2000);
    close_record_card(a);
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

/// Whether the Library shows no sheet, dialog or keyboard (a PDF fill's pages go in then: app.rs).
pub fn library_at_rest(a: &App) -> bool {
    a.lib.as_deref().is_none_or(|s| s.modal == Modal::None)
}

/// Whether the Library's own cards are in front with nothing over them: the Library is the screen, with no sheet,
/// no Files and no Graph up. Where a PDF KOReader handed over by its path opens its notebook once its import ends
/// (app.rs, book_tick); with anything else in front it opens nothing.
pub fn library_cards_in_front(a: &App) -> bool {
    a.scr == ScreenId::Library && a.lib.as_deref().is_none_or(|s| s.modal == Modal::None && !s.files.open && !s.map.open && !s.map_picker.open)
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
                .is_some_and(|h| matches!(h.id, H_RECORD_USE | H_RECORD_RETRY | H_RECORD_CANCEL | H_RECORD_DISABLE));
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
            .is_some_and(|h| matches!(h.id, H_RECORD_USE | H_RECORD_RETRY | H_RECORD_CANCEL | H_RECORD_DISABLE));
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
            app_toast(a, tr("Couldn't save gestures."), 3000);
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
        app_toast(a, tr("Couldn't save gestures."), 3000);
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

// The shortcuts again: shortcuts.ini's keys (and its buttons while there is no controller.ini), every
// device's own (root/shortcuts), and gestures.ini's gestures.
fn reload_shortcuts(a: &mut App) {
    app_shortcuts_load(a);
    app_gestures_load(a);
}

/// A controller's press while Settings > Shortcuts > Buttons learns one: true when it took it.
// A controller's press (ev) while the Buttons sheet learns: true when it was taken (the row's action has
// that button now, or the file could not be written, or the press was not the sheet's: a toast says so).
// It goes into the buttons the sheet shows (buttons_profile: a device's own, else controller.ini), and
// only when they are the ones the pressed controller uses (shortcut_profile_index): a press from another
// would be learned where that controller never looks, or change buttons it does not use.
pub fn library_learn_key(a: &mut App, ev: &InEvent) -> bool {
    let Some(s) = a.lib.as_deref() else {
        return false;
    };
    if s.modal != Modal::Buttons || s.buttons_group < 0 || s.learn_row < 0 {
        return false;
    }
    if !ev.gamepad {
        return true; // (a keyboard's key is shortcuts.ini's: nothing to learn here)
    }
    let code = ev.value;
    let action = learning_action(s);
    let from = shortcut_profile_index(&a.shortcut_profiles, ev.hid_name.as_str(), ev.hid_vendor, ev.hid_product);
    let shown = buttons_profile(a);
    if from != shown {
        // (the row still waits, for a press from a controller these buttons are)
        let name = if ev.hid_name.is_empty() {
            // TRANSLATORS: (Buttons sheet) the %s of the two toasts below for a controller that reports no
            // name.
            c_("buttons", "That controller")
        } else {
            ev.hid_name.as_str()
        };
        let mut msg = String::new();
        if from.is_some() {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the Buttons sheet did not
            // learn a press, as the controller pressed has its own shortcuts (Settings > Shortcuts, Own
            // shortcuts for this device) and the sheet shows others; %s is its own name. The Device row is the
            // Shortcuts sheet's ("Device: %s").
            csnprintf!(&mut msg, 200, tr("%s has buttons of its own: choose it in the Device row first."), name);
        } else {
            // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): the same for a controller
            // that uses the default buttons (controller.ini) while the sheet shows one device's own.
            csnprintf!(&mut msg, 200, tr("%s uses the default buttons: choose it in the Device row first."), name);
        }
        app_toast(a, &msg, 3000);
        lib(a).learn_at = now_ms();
        return true;
    }
    let cp = buttons_pads_path(a);
    let before = shortcuts_lookup(buttons_map(a), ActionSrc::Pad, code, 0);
    if shortcuts_learn(buttons_map_mut(a), &cp, ActionSrc::Pad, code, action) != 0 {
        buttons_save_failed(a);
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

// A Buttons row held (HOLD_MS): its actions lose their buttons, this row's alone, and it reads
// "none"; a toast says so. Its release starts no learning.
fn forget_row(a: &mut App, row: i32) {
    let Some(r) = binding_row_at(BUTTONS, lib(a).buttons_group as usize, row as usize) else { return };
    let mut msg = String::new();
    let cp = buttons_pads_path(a);
    lib(a).learn_row = -1;
    if shortcuts_forget(buttons_map_mut(a), &cp, ActionSrc::Pad, r.act[0], r.act[1]) != 0 {
        buttons_save_failed(a);
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

// A keyboard or controller the Shortcuts sheet lists: one connected now, or one with a profile saved.
#[derive(Clone, Default)]
struct ShortcutDev {
    slug: String,
    name: String,
    vendor: u16,
    product: u16,
    connected: bool,
    /// A controller connected now (false for a keyboard, and for a saved device not connected: not known)
    gamepad: bool,
}

// The Shortcuts sheet's devices: those connected now (input_hids: none while shortcuts are off), then those with
// a profile saved, each once (two of the same model and name are one device: they share a profile).
fn shortcut_devices(a: &App) -> Vec<ShortcutDev> {
    let mut h = [InputHid::default(); 4];
    let n = input_hids(&mut h).max(0) as usize;
    let mut out: Vec<ShortcutDev> = Vec::new();
    for hid in &h[..n.min(h.len())] {
        let slug = shortcut_profile_slug(hid.name.as_str(), hid.vendor, hid.product);
        match out.iter_mut().find(|d| d.slug == slug) {
            Some(d) => d.gamepad |= hid.gamepad,
            None => out.push(ShortcutDev {
                slug,
                name: hid.name.as_str().to_string(),
                vendor: hid.vendor,
                product: hid.product,
                connected: true,
                gamepad: hid.gamepad,
            }),
        }
    }
    for p in &a.shortcut_profiles {
        if !out.iter().any(|d| d.slug == p.slug) {
            out.push(ShortcutDev { slug: p.slug.clone(), name: p.name.clone(), vendor: p.vendor, product: p.product, connected: false, gamepad: false });
        }
    }
    out
}

// The Shortcuts sheet's device now: the one chosen while it is listed, else the first (a connected one before
// a saved one); None with none listed, and while shortcuts are off: no device is read then (input_hids), so
// whether a saved one is connected is not known.
fn shortcut_dev_now(a: &App) -> Option<ShortcutDev> {
    if !app_shortcuts_on(a) {
        return None;
    }
    let devs = shortcut_devices(a);
    let chosen = a.lib.as_deref().map_or("", |s| s.shortcut_dev.as_str());
    devs.iter().find(|d| d.slug == chosen).or(devs.first()).cloned()
}

// The device whose own buttons the Buttons sheet is about, as it opens: the Shortcuts sheet's device while its
// own shortcuts are on, unless it is a keyboard connected now (a controller's buttons never look in a
// keyboard's profile); "" for the default (controller.ini), as before profiles.
fn buttons_dev_for(a: &App) -> String {
    let Some(dev) = shortcut_dev_now(a) else { return String::new() };
    if dev.connected && !dev.gamepad {
        return String::new();
    }
    if a.shortcut_profiles.iter().any(|p| p.on && p.slug == dev.slug) {
        dev.slug
    } else {
        String::new()
    }
}

// The profile the Buttons sheet shows and learns into (buttons_dev, while its own shortcuts are on); None for
// the default (controller.ini).
fn buttons_profile(a: &App) -> Option<usize> {
    let slug = a.lib.as_deref()?.buttons_dev.as_str();
    if slug.is_empty() {
        return None;
    }
    a.shortcut_profiles.iter().position(|p| p.on && p.slug == slug)
}

// The toast for a Buttons sheet write that failed (the storage is full): the device's own file while the sheet
// is about one, else controller.ini.
fn buttons_save_failed(a: &mut App) {
    let msg = if buttons_profile(a).is_some() {
        tr("Couldn't save the device's shortcuts")
    } else {
        // TRANSLATORS: a toast: the Buttons sheet could not write its file (the storage is full).
        tr("Couldn't save controller.ini")
    };
    app_toast(a, msg, 4000);
}
fn buttons_map(a: &App) -> &Shortcuts {
    match buttons_profile(a) {
        Some(i) => &a.shortcut_profiles[i].map,
        None => &a.shortcuts,
    }
}
fn buttons_map_mut(a: &mut App) -> &mut Shortcuts {
    match buttons_profile(a) {
        Some(i) => &mut a.shortcut_profiles[i].map,
        None => &mut a.shortcuts,
    }
}
// The file the Buttons sheet writes: the profile's <slug>.controller.ini, else controller.ini.
fn buttons_pads_path(a: &App) -> String {
    match buttons_profile(a) {
        Some(i) => shortcut_profile_pads(&shortcut_profiles_dir(), &a.shortcut_profiles[i].slug),
        None => controller_path(),
    }
}

// The Shortcuts sheet's row for its device: its own shortcuts on, from a copy of the default the first time
// (shortcut_profile_create), or off (the profile kept, to be used again when they are turned on).
fn toggle_own_shortcuts(a: &mut App) {
    let Some(dev) = shortcut_dev_now(a) else { return };
    let dir = shortcut_profiles_dir();
    // (a profile by the device's slug, as listed: a saved one is turned on or off by its file's name, whatever
    // its header says)
    let r = match a.shortcut_profiles.iter().find(|p| p.slug == dev.slug) {
        Some(p) => shortcut_profile_set_on(&dir, &dev.slug, !p.on),
        None => {
            let sp = cformat!("%s/shortcuts.ini", &store_root());
            shortcut_profile_create(&dir, &dev.name, dev.vendor, dev.product, &sp, &controller_path())
        }
    };
    if r != 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold): the Shortcuts sheet could not write a device's
        // own shortcuts in grimoire/shortcuts (the storage is full, or shortcuts.ini cannot be read); also the
        // Buttons sheet's, for a button learned or forgotten in a device's own.
        app_toast(a, tr("Couldn't save the device's shortcuts"), 4000);
    }
    lib(a).shortcut_dev = dev.slug;
    reload_shortcuts(a);
    redraw(a, RefreshMode::Ui);
}

// The Pair sheet's status line: the controllers and keyboards Grimoire reads now.
fn hid_status(a: &mut App, out: &mut String, cap: usize) {
    let mut h = [InputHid::default(); 4];
    let n = input_hids(&mut h);
    let hid_changes = a.hid_changes;
    lib(a).shown_hid_changes = hid_changes;
    if n == 0 {
        // TRANSLATORS: the Pair sheet's status line: no controller or keyboard Grimoire reads now (about 1100
        // px at 44 px in bold, and the same for the two below); also the Shortcuts sheet's device row then.
        csnprintf!(
            out,
            cap,
            "%s",
            if app_shortcuts_on(a) {
                tr("No controller or keyboard connected")
            } else {
                ""
            }
        );
    } else if h[0].held {
        // TRANSLATORS: (Pair sheet) %s is the controller's own name, as it reports it: another app (a button
        // mapper) holds it, so Grimoire gets none of its presses.
        csnprintf!(out, cap, tr("%s: held by another app"), &h[0].name);
    } else {
        // TRANSLATORS: (Pair sheet) %s is the controller's or keyboard's own name.
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
        if s.modal == Modal::PairMenu || s.modal == Modal::Pair {
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
                app_toast(a, tr("Couldn't pair controller"), 3000);
            }
        }
        ASK_PAIR_STATUS => {
            if ok && r.pairing && (now_ms().wrapping_sub(s.pair_until) as i32) < 0 {
                bt_ask(s, ASK_PAIR_STATUS, 1000);
            } else {
                s.pairing = false;
                // TRANSLATORS: toasts as pairing a controller ends (Settings > Pair > Pair a controller).
                app_toast(
                    a,
                    if ok && !r.pairing {
                        // TRANSLATORS: a toast as pairing a controller ends (Settings > Pair > Pair a
                        // controller): it is paired, and connects a moment later.
                        tr("Paired.")
                    } else {
                        tr("Couldn't pair controller")
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
                    tr("Couldn't forget controller")
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

/// Settings' card and its measures are the app's design system, in ui.rs: the main page, every
/// submenu and the Display sheet draw the same cards and rows from these.
pub(crate) use crate::ui::{
    ui_card as settings_card, ui_row_gap as settings_gap, ui_row_pad as settings_pad,
};

/// A Settings card of segments (Pen feel's tools, its eraser choices): one outline around all of
/// them, a rule between each two, and the one picked filled—square to its neighbours, following the
/// card's rounded end where it has one. Returns where each segment is.
pub(crate) fn settings_strip(c: &mut Canvas, r: Rect, n: i32, on: i32) -> Vec<Rect> {
    let n = n.max(1);
    let w = r.w / n;
    let seg = |i: i32| -> Rect {
        let x = r.x + i * w;
        Rect { x, y: r.y, w: if i == n - 1 { r.x + r.w - x } else { w }, h: r.h }
    };
    if (0..n).contains(&on) {
        let s = seg(on);
        let rad = ui_px(16.0);
        if n == 1 {
            canvas_round_rect(c, s, rad, 0, true, 0);
        } else if on == 0 {
            canvas_round_rect(c, s, rad, 0, true, 0);
            canvas_fill(c, Rect { x: s.x + rad, y: s.y, w: s.w - rad, h: s.h }, 0);
        } else if on == n - 1 {
            canvas_fill(c, Rect { x: s.x, y: s.y, w: s.w - rad, h: s.h }, 0);
            canvas_round_rect(c, s, rad, 0, true, 0);
        } else {
            canvas_fill(c, s, 0);
        }
    }
    settings_card(c, r, ButtonStyle::Plain);
    for i in 1..n {
        canvas_vline(c, r.x + i * w, r.y, r.y + r.h, ui_px(3.0), 0);
    }
    (0..n).map(seg).collect()
}

pub(crate) use crate::ui::ui_switch as settings_switch;

/// Settings > Updates: one Update button that carries the version, and a switch for the branch the
/// updates come from (Preview: betas and releases; Stable: releases only). The check runs as the
/// page opens, so the version it shows is the feed's, not the last visit's.
fn draw_updates(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let preview = upd_preview(a);
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let page = settings_page(c, aw, ah, tr("Updates"), true, false);
    hits_add(&mut s.hits, page.back, H_SETTINGS_BACK, 0);
    hits_add(&mut s.hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    let avail = update_available();
    let (upd, newer, latest, why) = (s.upd, upd_newer(s), s.upd_latest.clone(), s.upd_why.clone());
    // The button says what the check found, and the version is on it either way; a failed check's
    // button is the way to ask again.
    let mut label = String::new();
    let st = if upd == UPD_CHECKING {
        csnprintf!(&mut label, 200, "%s", tr("Checking for updates..."));
        ButtonStyle::Disabled
    } else if !avail || (upd == UPD_FAILED && why == "no-kpm") {
        csnprintf!(&mut label, 200, "%s", tr("Updates come through KPM, which is not here."));
        ButtonStyle::Disabled
    } else if upd == UPD_DONE && newer {
        // TRANSLATORS: Settings > Updates' button when the check found a newer Grimoire (%s): a tap
        // closes Grimoire, KPM installs it, and Grimoire starts again.
        csnprintf!(&mut label, 200, tr("Update to %s"), &latest);
        ButtonStyle::Primary
    } else if upd == UPD_DONE {
        csnprintf!(&mut label, 200, tr("Grimoire %s is the newest."), GRIMOIRE_VERSION);
        ButtonStyle::Disabled
    } else if upd == UPD_FAILED && why == "network" {
        csnprintf!(&mut label, 200, "%s", tr("Could not check. Is Wi-Fi on?"));
        ButtonStyle::Plain
    } else if upd == UPD_FAILED {
        csnprintf!(&mut label, 200, "%s", tr("Could not check for updates."));
        ButtonStyle::Plain
    } else {
        csnprintf!(&mut label, 200, "%s", tr("Check for updates"));
        ButtonStyle::Plain
    };
    let br = Rect::new(in_.x, in_.y, in_.w, ui_px(150.0));
    ui_button(c, br, if newer { IconKind::Download } else { IconKind::Update }, &label, st);
    if st != ButtonStyle::Disabled {
        hits_add(&mut s.hits, br, H_UPD_NOW, 0);
    }
    // The branch, as a switch: on (Preview) for betas and releases, off (Stable) for releases only.
    let row = Rect::new(in_.x, br.y + br.h + ui_px(24.0), in_.w, ui_px(140.0));
    ui_frame(c, row, ui_px(16.0), 0, false, ui_px(3.0));
    // TRANSLATORS: Settings > Updates, the row whose switch picks the branch updates come from.
    font_draw_left(c, FontFace::Regular, ui_text_size(), rect_inflate(row, -ui_px(30.0)), tr("Update branch"), 0);
    settings_switch(c, row, preview);
    // TRANSLATORS: Settings > Updates, the branch the switch is on: Preview gets betas and releases,
    // Stable only releases.
    let branch = if preview { tr("Preview") } else { tr("Stable") };
    let bw = font_measure(FontFace::Bold, ui_text_size(), branch);
    let sx = row.x + row.w - ui_px(30.0) - ui_px(120.0);
    font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(sx - ui_px(24.0) - bw, row.y, bw, row.h), branch, 0);
    hits_add(&mut s.hits, row, H_UPD_CHANNEL, 0);
}

/// A Settings submenu's full screen: Back and the title above a rule, the list's rows below (its own
/// Done row left out: the page has an X at its top right instead) and an X at the top right.
fn draw_settings_menu(a: &mut App, title: &str, rows: &[MenuRow]) {
    let (aw, ah) = (a.w, a.h);
    let App {
        screen: c,
        lib,
        ..
    } = a;
    let s = lib.as_deref_mut().unwrap();
    let page = settings_page(c, aw, ah, title, true, false);
    hits_add(&mut s.hits, page.back, H_SETTINGS_BACK, 0);
    hits_add(&mut s.hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    let shown: Vec<&MenuRow> = rows
        .iter()
        .filter(|r| r.id != H_CANCEL && r.id != H_BUTTONS_DONE && r.id != H_GESTURES_DONE)
        .collect();
    // A submenu is the same page of rows as Settings' main page: the selection is filled, the rest
    // outlined, and a list longer than the screen is paged rather than squeezed.
    if s.settings_menu_title != title {
        s.settings_menu_title.clear();
        s.settings_menu_title.push_str(title);
        s.settings_menu_page = 0;
    }
    let list = ui_row_page(in_, shown.len(), s.settings_menu_page, ui_px(140.0), ui_px(90.0));
    s.settings_menu_page = list.page;
    for (r, row) in list.rows.iter().zip(shown[list.first..list.first + list.rows.len()].iter()) {
        let mut rr = UiRow::new(row.icon, &row.label);
        rr.style = row.st;
        rr.name = row.name;
        ui_row(c, *r, &rr);
        if row.st != ButtonStyle::Disabled {
            hits_add(&mut s.hits, *r, row.id, row.arg);
        }
    }
    ui_row_pager(c, &list);
    if list.page > 0 {
        hits_add(&mut s.hits, list.prev, H_SETTINGS_PAGE, -1);
    }
    if list.page + 1 < list.pages {
        hits_add(&mut s.hits, list.next, H_SETTINGS_PAGE, 1);
    }
}

// A vertical list of full-width buttons inside a sheet.
/// The sheet with its rows; returns where each row is.
fn draw_menu(a: &mut App, title: &str, rows: &[MenuRow]) -> Vec<Rect> {
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
    let mut rects = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let r = Rect::new(in_.x, in_.y + i as i32 * (rh + gap), in_.w, rh);
        rects.push(r);
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
        if row.name {
            ui_text_name(c, FontFace::Bold, ui_text_size(), ui_small_size(), text_rect, &row.label, 2, fg);
        } else if row.id == H_NONE && row.st == ButtonStyle::Disabled {
            ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), text_rect, &row.label, 2, false, fg);
        } else {
            font_draw_left(c, FontFace::Bold, ui_text_size(), text_rect, &row.label, fg);
        }
        if row.st != ButtonStyle::Disabled {
            hits_add(&mut s.hits, r, row.id, row.arg);
        }
    }
    rects
}

// ---- Settings > Wi-Fi (2.1.0-beta.2) ----

/// The radio row's three taps (H_WIFI_RADIO's arg).
const WIFI_RETRY: i32 = 0;
const WIFI_ON: i32 = 1;
const WIFI_OFF: i32 = 2;
/// Settings > Wi-Fi's three views: the scan's networks first, the saved ones, and Other...'s hidden
/// network.
const WIFI_VIEW_NEARBY: usize = 0;
const WIFI_VIEW_SAVED: usize = 1;
const WIFI_VIEW_OTHER: usize = 2;
/// While the nearby page is idle, it asks for a quiet scan this often.
const WIFI_REFRESH_MS: u32 = 20_000;
/// The badge on a WPA2/WPA3-Enterprise row (a product label, the same in every language).
const WIFI_BETA: &str = "Beta";

/// A drag in the Wi-Fi list: where it went down, the scroll it began at, and whether it has passed
/// the slop and is scrolling.
#[derive(Clone, Copy, Debug, Default)]
struct WifiScroll {
    x0: i32,
    y0: i32,
    at: i32,
    on: bool,
}

/// What the Wi-Fi submenu was drawn from: drawn again when any of it changes.
#[derive(Clone, Default, PartialEq)]
struct WifiShown {
    status: Option<WifiStatus>,
    busy: bool,
    failed: bool,
    current: Option<Vec<u8>>,
    saved: Option<Vec<Vec<u8>>>,
    nearby: Option<Vec<WifiNetwork>>,
    scanning: bool,
    scan_failed: bool,
    add_failed: bool,
    delete_failed: bool,
    disconnect_failed: bool,
    join: Option<(Vec<u8>, WifiJoin)>,
}

/// Settings > Wi-Fi: nearby networks first, with Rescan, Other... and Saved networks above them. It
/// reads only while it is open, and is all a full-page Settings submenu: the radio and what is
/// connected, the scan's networks, the saved networks with a trash button each, and a hidden
/// network's name and security. Opening it starts a scan once the radio is on.
#[derive(Default)]
struct WifiSheet {
    check_at: u32,
    watch_at: u32,
    /// everything was asked for since the submenu opened (not while the Share card's last read still ran);
    /// the saved networks once more as the radio came on (wifid may not answer while it is off), or after a
    /// switch the helper refused
    asked: bool,
    relisted: bool,
    reread: bool,
    /// WIFI_VIEW_*: Nearby, Saved, or Other...
    view: usize,
    /// how far the list is scrolled down, in pixels; rows holds every saved name and near every scan
    /// result, not just the visible ones (H_WIFI_NET/H_WIFI_NEAR's arg is an index into them)
    scroll: i32,
    scroll_max: i32,
    /// the list's last drawn viewport, and a drag in it
    list: Rect,
    scroll_drag: Option<WifiScroll>,
    rows: Vec<Vec<u8>>,
    near: Vec<WifiNetwork>,
    /// the network whose password or enterprise username is being typed, and a hidden network's name
    /// while Other... is open. The enterprise username waits here between its keyboard and the
    /// password's; it is wiped when the sheet resumes.
    add: Option<WifiNetwork>,
    eap_user: Option<Vec<u8>>,
    other: Option<Vec<u8>>,
    /// a scan is due as soon as the radio is on (opening the sheet, turning it on, or adding a network)
    scan_auto: bool,
    /// when the nearby list was last scanned, and a quiet refresh is in flight whose 20 s wait starts
    /// when its answer comes back
    refresh_at: u32,
    refresh_wait: bool,
    shown: Option<WifiShown>,
}

fn wifi_shown() -> WifiShown {
    let status = wifi_status();
    WifiShown {
        status,
        busy: wifi_changing() || (wifi_busy() && status.is_none()),
        failed: wifi_failed(),
        current: wifi_current().filter(|_| status.is_some_and(|st| st.connected == Some(true))),
        saved: wifi_saved(),
        nearby: wifi_nearby(),
        // (a quiet refresh keeps the list and its Rescan button as they are)
        scanning: wifi_scanning() && !wifi_scan_quieting(),
        scan_failed: wifi_scan_failed(),
        add_failed: wifi_add_failed(),
        delete_failed: wifi_delete_failed(),
        disconnect_failed: wifi_disconnect_failed(),
        join: wifi_joining(),
    }
}

fn wifi_sheet_open(a: &mut App) {
    let now = now_ms();
    wifi_join_clear();
    let asked = wifi_request(WifiAction::Networks);
    lib(a).wifi = WifiSheet {
        check_at: now,
        watch_at: now,
        asked,
        view: WIFI_VIEW_NEARBY,
        scan_auto: true,
        refresh_at: now,
        ..WifiSheet::default()
    };
    open_modal(a, Modal::Wifi);
}

/// Share's Wi-Fi button opens the same Settings > Wi-Fi submenu; closing it returns to Share.
pub(crate) fn library_open_wifi_from_share(a: &mut App) {
    if a.scr != ScreenId::Library {
        app_switch(a, ScreenId::Library);
    }
    let s = lib(a);
    s.share_wifi_return = true;
    s.settings_return_editor = false;
    wifi_sheet_open(a);
}

/// Whether the Wi-Fi submenu on screen was opened from Share (the app probe's wifi.open).
pub(crate) fn library_wifi_from_share(a: &App) -> bool {
    a.lib
        .as_deref()
        .is_some_and(|s| s.share_wifi_return && s.modal == Modal::Wifi)
}

/// Back from the password keyboard: the Wi-Fi page stays open, its nearby list fresh.
fn wifi_sheet_resume(a: &mut App) {
    let now = now_ms();
    let parent = lib(a).kb_parent;
    let s = lib(a);
    s.modal = if matches!(parent, Modal::None | Modal::Kb) { Modal::Wifi } else { parent };
    s.kb_parent = Modal::None;
    s.wifi.add = None;
    if let Some(mut u) = s.wifi.eap_user.take() {
        u.fill(0);
    }
    s.wifi.other = None;
    s.wifi.view = WIFI_VIEW_NEARBY;
    s.wifi.scroll = 0;
    s.wifi.scroll_drag = None;
    s.wifi.scan_auto = true;
    s.wifi.refresh_at = now;
    s.wifi.refresh_wait = false;
    s.wifi.check_at = now;
    s.wifi.watch_at = now;
    redraw(a, RefreshMode::Ui);
}

fn wifi_security_name(sec: WifiSecurity) -> &'static str {
    match sec {
        WifiSecurity::Open => tr("Open"),
        WifiSecurity::Wep => "WEP",
        WifiSecurity::Wpa => "WPA",
        WifiSecurity::Wpa2 => "WPA2",
        WifiSecurity::Wpa3 => "WPA3",
        WifiSecurity::Owe => "OWE",
        WifiSecurity::Enterprise => tr("Enterprise"),
        WifiSecurity::Enterprise192 => tr("Enterprise 192-bit"),
        WifiSecurity::Unknown => tr("Unknown"),
    }
}

/// The three bars a scan's signal fills: a third, two thirds, or all three.
fn wifi_bars(signal: i32, signal_max: i32) -> i32 {
    if signal <= 0 || signal_max <= 0 {
        return 0;
    }
    let pct = (signal as i64 * 100 / signal_max as i64).clamp(0, 100);
    if pct >= 70 {
        3
    } else if pct >= 35 {
        2
    } else {
        1
    }
}

/// A lock drawn from its shackle and body: the icon font's lock glyph is not in every face.
fn draw_wifi_lock(c: &mut Canvas, r: Rect, fg: u8, bg: u8) {
    let shackle = Rect::new(r.x + ui_px(7.0), r.y + ui_px(2.0), r.w - ui_px(14.0), r.h - ui_px(12.0));
    canvas_round_rect(c, shackle, ui_px(8.0), fg, false, ui_px(3.0));
    let body = Rect::new(r.x + ui_px(2.0), r.y + ui_px(14.0), r.w - ui_px(4.0), r.h - ui_px(16.0));
    canvas_round_rect(c, body, ui_px(3.0), fg, true, 0);
    let hole = Rect::new(r.x + r.w / 2 - ui_px(2.0), r.y + ui_px(18.0), ui_px(4.0), ui_px(7.0));
    canvas_round_rect(c, hole, ui_px(1.0), bg, true, 0);
}

/// The Wi-Fi signal beside a network: three bars, the lit ones its strength, and a small lock over
/// their lower right when the network is not open. `signal` None draws all three unlit (a saved
/// network no scan has measured).
fn draw_wifi_icon(c: &mut Canvas, r: Rect, signal: Option<(i32, i32)>, secure: bool, fg: u8, card_bg: u8) {
    let gap = ui_px(5.0);
    let bw = (r.w * 16 / 100).max(ui_px(5.0));
    let base = r.y + r.h - ui_px(9.0);
    let heights = [r.h * 32 / 100, r.h * 52 / 100, r.h * 72 / 100];
    let lit = signal.map_or(0, |(s, m)| wifi_bars(s, m));
    for (i, h) in heights.iter().enumerate() {
        let x = r.x + ui_px(5.0) + i as i32 * (bw + gap);
        let bar = Rect::new(x, base - *h, bw, *h);
        let gray = if (i as i32) < lit {
            fg
        } else if fg == 0 {
            165
        } else {
            95
        };
        ui_frame(c, bar, ui_px(3.0), gray, true, 0);
    }
    if secure {
        let lr = Rect::new(
            r.x + r.w - ui_px(30.0),
            r.y + r.h - ui_px(33.0),
            ui_px(31.0),
            ui_px(31.0),
        );
        ui_frame(c, rect_inflate(lr, ui_px(3.0)), ui_px(4.0), card_bg, true, 0);
        draw_wifi_lock(c, lr, fg, card_bg);
    }
}

/// One network row: its signal and lock at the left, its name, what the Kindle knows of it under
/// that, and a saved network's trash button at the right.
struct WifiRow<'a> {
    name: &'a str,
    security: Option<WifiSecurity>,
    signal: Option<(i32, i32)>,
    saved: bool,
    current: bool,
    enabled: bool,
    id: i32,
    arg: i32,
    /// H_WIFI_DELETE's arg for a saved row's trash button, None for no button
    trash: Option<i32>,
}

fn wifi_beta(sec: Option<WifiSecurity>) -> bool {
    sec == Some(WifiSecurity::Enterprise)
}

/// A network row's small subtitle: what the Kindle knows of it, then a Beta badge after the
/// security on an Enterprise (EAP) row only. `right` is the row's text limit, before its trash.
fn draw_wifi_subtitle(c: &mut Canvas, tx: i32, right: i32, y: i32, h: i32, sub: &str, beta: bool, fg: u8, card_bg: u8) {
    let right = right.max(tx);
    let px = ui_small_size() * 0.8;
    let badge_h = ui_px(36.0).min(h);
    let gap = ui_px(12.0);
    let badge_w = if beta {
        ui_chip_w(FontFace::Bold, px, WIFI_BETA, badge_h, 0, right - tx)
    } else {
        0
    };
    let room = (right - tx - if beta { badge_w + gap } else { 0 }).max(0);
    ui_text(
        c,
        FontFace::Regular,
        ui_small_size(),
        px,
        Rect::new(tx, y, room, h),
        sub,
        1,
        false,
        fg,
    );
    if beta {
        let used = font_measure(FontFace::Regular, px, sub).min(room);
        let br = Rect::new(
            (tx + used + gap).min(right - badge_w),
            y + (h - badge_h) / 2,
            badge_w,
            badge_h,
        );
        ui_frame(c, br, badge_h / 2, fg, true, 0);
        font_draw_centered(c, FontFace::Bold, px, br, WIFI_BETA, card_bg);
    }
}

fn draw_wifi_row(c: &mut Canvas, hits: &mut HitList, r: Rect, row: &WifiRow) {
    let st = if row.current {
        ButtonStyle::Selected
    } else if row.enabled {
        ButtonStyle::Plain
    } else {
        ButtonStyle::Disabled
    };
    let fg: u8 = if row.current {
        255
    } else if row.enabled {
        0
    } else {
        170
    };
    let card_bg: u8 = if row.current { 0 } else { 255 };
    settings_card(c, r, st);
    // The connected card is filled black: a white rule inside its edge sets it apart without
    // changing the white bars of its signal.
    if row.current {
        ui_frame(c, rect_inflate(r, -ui_px(7.0)), ui_px(10.0), 255, false, ui_px(4.0));
    }
    // (before the trash, so the trash is the hit over the row it sits in)
    if row.enabled && row.id != H_NONE {
        hits_add(hits, r, row.id, row.arg);
    }
    let isz = (r.h - ui_px(20.0)).clamp(ui_px(64.0), ui_px(90.0));
    let ir = Rect::new(r.x + settings_pad(), r.y + (r.h - isz) / 2, isz, isz);
    let secure = row.security.is_some_and(|s| s != WifiSecurity::Open);
    draw_wifi_icon(c, ir, row.signal, secure, fg, card_bg);
    let mut right = r.x + r.w - settings_pad();
    if let Some(arg) = row.trash {
        let bs = (r.h - ui_px(30.0)).clamp(ui_px(70.0), ui_px(96.0));
        let br = Rect::new(right - bs, r.y + (r.h - bs) / 2, bs, bs);
        ui_button(c, br, IconKind::Trash, "", ButtonStyle::Plain);
        // (after the row, so the trash is the hit over it)
        hits_add(hits, br, H_WIFI_DELETE, arg);
        right = br.x - ui_px(18.0);
    }
    let tx = ir.x + ir.w + settings_gap();
    let mut sub = String::new();
    if row.current {
        sub.push_str(tr("Connected"));
    } else if row.saved {
        sub.push_str(tr("Saved"));
    }
    if let Some(sec) = row.security {
        if !sub.is_empty() {
            sub.push_str(" \u{b7} ");
        }
        sub.push_str(wifi_security_name(sec));
    }
    let text_w = (right - tx).max(0);
    if sub.is_empty() {
        ui_text_name(c, FontFace::Bold, ui_text_size(), ui_small_size(), Rect::new(tx, r.y, text_w, r.h), row.name, 1, fg);
    } else {
        ui_text_name(
            c,
            FontFace::Bold,
            ui_text_size(),
            ui_small_size(),
            Rect::new(tx, r.y + ui_px(12.0), text_w, r.h - ui_px(56.0)),
            row.name,
            1,
            fg,
        );
        draw_wifi_subtitle(
            c,
            tx,
            right,
            r.y + r.h - ui_px(54.0),
            ui_px(42.0),
            &sub,
            wifi_beta(row.security),
            fg,
            card_bg,
        );
    }
}

fn draw_wifi_message(c: &mut Canvas, r: Rect, text: &str) {
    settings_card(c, r, ButtonStyle::Disabled);
    ui_text(
        c,
        FontFace::Regular,
        ui_text_size(),
        ui_small_size(),
        rect_inflate(r, -settings_pad()),
        text,
        2,
        false,
        170,
    );
}

/// One row's height: nine rows' worth of the list, within the touch and density bounds, so a
/// drag moves by the same measure in every orientation.
fn wifi_row_h(list_h: i32) -> i32 {
    let gap = ui_px(14.0);
    ((list_h - 8 * gap) / 9).clamp(ui_px(92.0), ui_px(154.0))
}

/// The rects of n rows, the first `scroll` pixels above the list's top: rows outside the list
/// are still in the vector (their caller clips them).
fn wifi_row_rects(in_: Rect, n: usize, scroll: i32) -> Vec<Rect> {
    let gap = ui_px(14.0);
    let rh = wifi_row_h(in_.h);
    (0..n)
        .map(|i| Rect::new(in_.x, in_.y + i as i32 * (rh + gap) - scroll, in_.w, rh))
        .collect()
}

/// The height of n rows with their gaps, for the list's greatest scroll.
fn wifi_content_h(n: usize, list_h: i32) -> i32 {
    if n == 0 {
        return 0;
    }
    let gap = ui_px(14.0);
    n as i32 * wifi_row_h(list_h) + (n as i32 - 1) * gap
}

/// Keeps the hits added while drawing the list inside its viewport (a row cut by the list's top
/// or foot must not be tappable outside it).
fn wifi_clip_hits(hits: &mut HitList, from: usize, clip: Rect) {
    let mut i = from;
    while i < hits.h.len() {
        let r = rect_intersect(hits.h[i].r, clip);
        if rect_is_empty(r) {
            hits.h.remove(i);
        } else {
            hits.h[i].r = r;
            i += 1;
        }
    }
}

/// A name without the square brackets the Kindle puts around the connected profile. The brackets
/// are a marker, not part of the network's name; an empty pair is left alone.
fn wifi_unwrapped(name: &[u8]) -> &[u8] {
    match name.strip_prefix(b"[").and_then(|n| n.strip_suffix(b"]")) {
        Some(inner) if !inner.is_empty() => inner,
        _ => name,
    }
}

/// Whether `name` is the network the Kindle is connected to, allowing for the brackets the
/// Kindle may put around the connected profile's name.
fn wifi_is_current(name: &[u8], current: Option<&[u8]>) -> bool {
    current.is_some_and(|c| wifi_unwrapped(c) == wifi_unwrapped(name))
}

/// The saved rows to show: the Kindle's order, with no connected network moved to the top. When
/// the Kindle lists a bracketed name and its plain name as saved, the bracketed one is the
/// connected marker, not a second profile. Keep the plain profile for its actions; a bracketed
/// name with no plain form is kept (its label is still drawn without the brackets).
fn wifi_saved_rows(saved: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut labels: Vec<&[u8]> = Vec::new();
    for name in saved {
        let label = wifi_unwrapped(name);
        if label.len() != name.len() && saved.iter().any(|other| other.as_slice() == label) {
            continue;
        }
        if labels.contains(&label) {
            continue;
        }
        labels.push(label);
        rows.push(name.clone());
    }
    rows
}

/// Settings > Wi-Fi: the radio's state and the three controls over whichever list is open.
fn draw_wifi_sheet(a: &mut App) {
    if lib(a).wifi.view == WIFI_VIEW_OTHER {
        draw_wifi_other_page(a);
        return;
    }
    let w = wifi_shown();
    let auto_scan = lib(a).wifi.scan_auto;
    let name = |n: &[u8]| String::from_utf8_lossy(wifi_unwrapped(n)).into_owned();
    let mut status = String::new();
    let mut status_named = false;
    let on = w.status.is_some_and(|st| st.wireless && st.enabled);
    match &w.join {
        Some((n, WifiJoin::Pending)) => {
            // TRANSLATORS: (Settings > Wi-Fi) its status line, 1470 px at 44 px in bold or two lines at 34 px; %s
            // is a Wi-Fi network's name as the Kindle saved it (shortened if it is long). Connecting: a switch
            // to that network is under way; Could not connect: it did not connect in 45 seconds, or the Kindle
            // refused it; Connected to: the network the Kindle is connected to.
            csnprintf!(&mut status, 200, tr("Connecting to %s…"), &name(n));
            status_named = true;
        }
        Some((n, WifiJoin::Failed)) if !w.busy => {
            csnprintf!(&mut status, 200, tr("Could not connect to %s."), &name(n));
            status_named = true;
        }
        _ => {}
    }
    if status.is_empty() {
        let text = if w.busy {
            tr("Checking Wi-Fi…")
        } else if w.failed {
            if wifi_add_failed() {
                tr("Could not add the network.")
            } else if wifi_delete_failed() {
                // TRANSLATORS: (Settings > Wi-Fi) the status when a saved network's profile could not be deleted.
                tr("Could not forget the network.")
            } else {
                tr("Could not read or change Wi-Fi.")
            }
        } else if w.disconnect_failed {
            // (the helper failed to drop the link; the status still describes the connection it kept)
            tr("Could not disconnect.")
        } else {
            match w.status {
                Some(_) if !on => tr("Wi-Fi is off"),
                Some(st) if st.connected == Some(true) => {
                    if let Some(n) = &w.current {
                        csnprintf!(&mut status, 200, tr("Connected to %s"), &name(n));
                        status_named = true;
                    }
                    tr("Connected to Wi-Fi")
                }
                Some(st) if st.connected == Some(false) => tr("Wi-Fi is on; no connection yet"),
                Some(_) => tr("Wi-Fi is on; connection unknown"),
                None => tr("Wi-Fi unavailable"),
            }
        };
        if status.is_empty() {
            status.push_str(text);
        }
    }
    let (radio, arg, can) = match w.status {
        None => (tr("Retry"), WIFI_RETRY, !w.busy && wifi_available()),
        Some(_) if on => (tr("Turn off Wi-Fi"), WIFI_OFF, !w.busy),
        Some(_) => (tr("Turn on Wi-Fi"), WIFI_ON, !w.busy),
    };
    let style = |can: bool| if can { ButtonStyle::Plain } else { ButtonStyle::Disabled };
    let view = lib(a).wifi.view.min(WIFI_VIEW_SAVED);
    let (aw, ah) = (a.w, a.h);
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    // TRANSLATORS: the title of Settings > Wi-Fi.
    let page = settings_page(c, aw, ah, tr("Wi-Fi"), true, false);
    hits_add(&mut s.hits, page.back, H_SETTINGS_BACK, 0);
    hits_add(&mut s.hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    let status_h = ui_px(110.0);
    let gap = ui_px(16.0);
    let radio_w = ui_px(390.0).min(in_.w / 2);
    let status_r = Rect::new(in_.x, in_.y, in_.w - radio_w - gap, status_h);
    ui_text(
        c,
        if status_named { FontFace::Bold } else { FontFace::Regular },
        ui_text_size(),
        ui_small_size(),
        status_r,
        &status,
        2,
        false,
        0,
    );
    let radio_r = Rect::new(in_.x + in_.w - radio_w, in_.y, radio_w, status_h);
    ui_button(c, radio_r, IconKind::Share, radio, style(can));
    if can {
        hits_add(&mut s.hits, radio_r, H_WIFI_RADIO, arg);
    }
    let by = in_.y + status_h + ui_px(18.0);
    let bh = ui_px(130.0);
    let bw = (in_.w - 2 * gap) / 3;
    let scan_r = Rect::new(in_.x, by, bw, bh);
    let other_r = Rect::new(in_.x + bw + gap, by, bw, bh);
    let saved_r = Rect::new(in_.x + 2 * (bw + gap), by, in_.w - 2 * (bw + gap), bh);
    let scan_can = on && !w.scanning && !w.busy;
    // TRANSLATORS: (Settings > Wi-Fi) the button above the nearby networks that scans again.
    ui_button(c, scan_r, IconKind::Search, tr("Rescan"), style(scan_can));
    if scan_can {
        hits_add(&mut s.hits, scan_r, H_WIFI_SCAN, 0);
    }
    // TRANSLATORS: (Settings > Wi-Fi) the button above the networks that adds a hidden network by name.
    ui_button(c, other_r, IconKind::Plus, tr("Other..."), ButtonStyle::Plain);
    hits_add(&mut s.hits, other_r, H_WIFI_OTHER, 0);
    // TRANSLATORS: (Settings > Wi-Fi) the button above the networks that shows the Kindle's saved ones;
    // tapped while they are shown, it goes back to the nearby ones.
    ui_button(
        c,
        saved_r,
        IconKind::Check,
        tr("Saved networks"),
        if view == WIFI_VIEW_SAVED { ButtonStyle::Selected } else { ButtonStyle::Plain },
    );
    hits_add(&mut s.hits, saved_r, H_WIFI_TAB, WIFI_VIEW_SAVED as i32);
    let list = Rect::new(
        in_.x,
        by + bh + ui_px(22.0),
        in_.w,
        (ah - ui_px(36.0) - (by + bh + ui_px(22.0))).max(0),
    );
    let mut info: Option<String> = None;
    let mut all_nets: Vec<WifiNetwork> = Vec::new();
    let mut all_names: Vec<Vec<u8>> = Vec::new();
    if view == WIFI_VIEW_NEARBY {
        // The networks a scan found, the connected one first, then the saved ones, then by signal.
        let mut networks = w.nearby.clone().unwrap_or_default();
        networks.sort_by(|a, b| {
            let key = |n: &WifiNetwork| {
                let here = wifi_is_current(&n.essid, w.current.as_deref());
                let saved = w.saved.as_ref().is_some_and(|s| s.iter().any(|x| x == &n.essid));
                // (no signal key: a 20 s refresh must not reorder the list as strength changes)
                (!here, !saved)
            };
            key(a).cmp(&key(b))
        });
        if !on {
            // TRANSLATORS: (Settings > Wi-Fi) the nearby list while the radio is off.
            info = Some(tr("Turn on Wi-Fi to see nearby networks.").to_string());
        } else if w.scanning || (auto_scan && w.nearby.is_none()) {
            // TRANSLATORS: (Settings > Wi-Fi) the nearby list while its scan runs.
            info = Some(tr("Scanning for networks…").to_string());
        } else if w.scan_failed {
            // TRANSLATORS: (Settings > Wi-Fi) the nearby list when the last scan could not run or its answer
            // was not one this reader knows.
            info = Some(tr("Could not scan for networks.").to_string());
        } else if w.nearby.as_ref().is_some_and(|v| v.is_empty()) {
            // TRANSLATORS: (Settings > Wi-Fi) the nearby list when no network is near.
            info = Some(tr("No networks found.").to_string());
        } else if w.nearby.is_none() {
            // TRANSLATORS: (Settings > Wi-Fi) the nearby list before its first scan has an answer.
            info = Some(tr("Tap Rescan to look for networks.").to_string());
        } else {
            all_nets = networks;
        }
    } else {
        // The saved networks, in the Kindle's order, each with its trash button. The connected one
        // keeps its place and is drawn as the Connected card there, not moved to the top.
        let names = wifi_saved_rows(w.saved.as_deref().unwrap_or_default());
        if w.saved.is_none() {
            let why = if on {
                // TRANSLATORS: (Settings > Wi-Fi) a line in place of the saved networks when they could not be read.
                tr("Saved networks could not be read.")
            } else {
                // TRANSLATORS: (Settings > Wi-Fi) a line in place of the saved networks while the radio is off.
                tr("Turn on Wi-Fi to see the saved networks.")
            };
            info = Some(why.to_string());
        } else if names.is_empty() {
            // TRANSLATORS: (Settings > Wi-Fi) the Kindle has no saved Wi-Fi network; Grimoire adds none itself.
            info = Some(tr("No saved networks. Add one with Other... or in Kindle settings.").to_string());
        } else {
            all_names = names;
        }
    }
    let total = all_nets.len() + all_names.len();
    let scroll_max = (wifi_content_h(total, list.h) - list.h).max(0);
    let scroll = s.wifi.scroll.clamp(0, scroll_max);
    if let Some(text) = info {
        draw_wifi_message(c, Rect::new(list.x, list.y, list.w, wifi_row_h(list.h)), &text);
    } else {
        let rects = wifi_row_rects(list, total, scroll);
        let old_clip = c.clip;
        canvas_set_clip(c, rect_intersect(old_clip, list));
        let hit_from = s.hits.h.len();
        for (i, net) in all_nets.iter().enumerate() {
            let r = rects[i];
            if rect_is_empty(rect_intersect(r, list)) {
                continue;
            }
            let saved = w.saved.as_ref().is_some_and(|s| s.iter().any(|x| x == &net.essid));
            let label = name(&net.essid);
            let row = WifiRow {
                name: &label,
                security: Some(net.security),
                signal: Some((net.signal, net.signal_max)),
                saved,
                // The network connected to is drawn as the Connected card here too, as in the saved
                // list (the Kindle may bracket the connected profile's name).
                current: wifi_is_current(&net.essid, w.current.as_deref()),
                enabled: net.security != WifiSecurity::Unknown || saved,
                id: H_WIFI_NEAR,
                arg: i as i32,
                trash: None,
            };
            draw_wifi_row(c, &mut s.hits, r, &row);
        }
        for (i, saved_name) in all_names.iter().enumerate() {
            let r = rects[all_nets.len() + i];
            if rect_is_empty(rect_intersect(r, list)) {
                continue;
            }
            let nearby = w.nearby.as_ref().and_then(|v| v.iter().find(|n| &n.essid == saved_name));
            let label = name(saved_name);
            let here = wifi_is_current(saved_name, w.current.as_deref());
            let row = WifiRow {
                name: &label,
                security: nearby.map(|n| n.security),
                signal: nearby.map(|n| (n.signal, n.signal_max)),
                saved: true,
                current: here,
                enabled: !w.busy,
                id: H_WIFI_NET,
                arg: i as i32,
                trash: (!w.busy).then_some(i as i32),
            };
            draw_wifi_row(c, &mut s.hits, r, &row);
        }
        canvas_set_clip(c, old_clip);
        wifi_clip_hits(&mut s.hits, hit_from, list);
    }
    let ws = &mut s.wifi;
    ws.scroll = scroll;
    ws.scroll_max = scroll_max;
    ws.list = list;
    ws.rows = all_names;
    ws.near = all_nets;
    ws.shown = Some(w);
}

/// Ask for a hidden or scanned network's password, on the masked keyboard, then `wifi_add` it with
/// `security` (WPA/WPA2, or WPA3-Personal).
fn wifi_password_start(a: &mut App, n: Vec<u8>, security: WifiSecurity) {
    let mut title = String::new();
    let shown = String::from_utf8_lossy(&n).into_owned();
    // TRANSLATORS: the password keyboard's title for a Wi-Fi network; %s is its name.
    csnprintf!(&mut title, 200, tr("Password for %s"), &shown);
    lib(a).wifi.add = Some(WifiNetwork {
        essid: n,
        security,
        signal: 0,
        signal_max: 1,
        known: false,
    });
    open_kb_secret(a, KbPurpose::WifiPassword, &title);
}

/// Start the username then password flow for a WPA2/WPA3-Enterprise network. The username is typed
/// on the normal keyboard, the password on the masked one; both live only in WifiSheet until the add.
fn wifi_eap_start(a: &mut App, n: WifiNetwork) {
    let mut title = String::new();
    let shown = String::from_utf8_lossy(&n.essid).into_owned();
    // TRANSLATORS: the keyboard's title for a school/enterprise Wi-Fi username; %s is its name.
    csnprintf!(&mut title, 200, tr("Username for %s"), &shown);
    let ws = &mut lib(a).wifi;
    ws.add = Some(n);
    if let Some(mut u) = ws.eap_user.take() {
        u.fill(0);
    }
    open_kb(a, KbPurpose::WifiEapUser, &title, b"");
}

/// Other...'s page: the typed hidden network's name, then whether it is open or takes a password.
fn draw_wifi_other_page(a: &mut App) {
    let name = lib(a).wifi.other.clone().unwrap_or_default();
    let mut label = String::new();
    let shown = String::from_utf8_lossy(&name).into_owned();
    // TRANSLATORS: (Settings > Wi-Fi > Other...) the hidden network whose security is being picked; %s is its name.
    csnprintf!(&mut label, 240, tr("Network: %s"), &shown);
    let rows = vec![
        MenuRow {
            name: true,
            ..menu_row(IconKind::None, &label, H_NONE, 0, ButtonStyle::Disabled)
        },
        // TRANSLATORS: (Settings > Wi-Fi > Other...) the choice for a hidden network with no password.
        menu_row(IconKind::None, tr("Open network"), H_WIFI_OTHER_OPEN, 0, ButtonStyle::Plain),
        // TRANSLATORS: (Settings > Wi-Fi > Other...) the choice for a hidden WPA or WPA2 network.
        menu_row(IconKind::None, tr("WPA/WPA2 password"), H_WIFI_OTHER_WPA, 0, ButtonStyle::Plain),
        // TRANSLATORS: (Settings > Wi-Fi > Other...) the choice for a hidden WPA3-Personal network.
        menu_row(IconKind::None, tr("WPA3 password"), H_WIFI_OTHER_WPA3, 0, ButtonStyle::Plain),
        // TRANSLATORS: (Settings > Wi-Fi > Other...) the choice for a hidden WPA2- or WPA3-Enterprise network.
        menu_row(IconKind::None, tr("WPA2/WPA3-Enterprise"), H_WIFI_OTHER_EAP, 0, ButtonStyle::Plain),
        menu_row(IconKind::Close, tr("Cancel"), H_WIFI_OTHER_CANCEL, 0, ButtonStyle::Plain),
    ];
    // TRANSLATORS: the title of the page that adds a hidden Wi-Fi network by name.
    draw_settings_menu(a, tr("Other network"), &rows);
}

/// A tap on the connected row: drop that network and leave the radio on.
fn wifi_disconnect_tapped(a: &mut App) {
    if wifi_disconnect() {
        // TRANSLATORS: (Settings > Wi-Fi) the toast while the network connected to is dropped.
        app_toast(a, tr("Disconnecting…"), 2000);
    } else {
        // TRANSLATORS: (Settings > Wi-Fi) the toast when the network cannot be disconnected now.
        app_toast(a, tr("Could not disconnect."), 3000);
    }
}

/// A drag in the Wi-Fi list scrolls it, while a tap still reaches the row under the finger.
fn wifi_sheet_scroll(a: &mut App, ph: i32, ev: &InEvent) -> bool {
    if ph == 0 {
        let s = lib(a);
        let in_list = s.wifi.view != WIFI_VIEW_OTHER
            && rect_contains(s.wifi.list, ev.x as i32, ev.y as i32);
        s.wifi.scroll_drag = in_list.then_some(WifiScroll {
            x0: ev.x as i32,
            y0: ev.y as i32,
            at: s.wifi.scroll,
            on: false,
        });
        return false;
    }
    let Some(mut d) = lib(a).wifi.scroll_drag else {
        return false;
    };
    if ph == 1 {
        let mut repaint = false;
        if !d.on {
            let dx = (ev.x as i32 - d.x0).abs();
            let dy = (ev.y as i32 - d.y0).abs();
            if dy < ui_px(30.0) || dy <= dx {
                return false;
            }
            d.on = true;
            lib(a).pressed = None;
            repaint = true;
        }
        let next = (d.at + d.y0 - ev.y as i32).clamp(0, lib(a).wifi.scroll_max);
        if next != lib(a).wifi.scroll {
            lib(a).wifi.scroll = next;
            repaint = true;
        }
        lib(a).wifi.scroll_drag = Some(d);
        if repaint {
            redraw(a, RefreshMode::Fast);
        }
        true
    } else if d.on {
        lib(a).wifi.scroll_drag = None;
        lib(a).pressed = None;
        redraw(a, RefreshMode::Ui);
        true
    } else {
        lib(a).wifi.scroll_drag = None;
        false
    }
}

fn wifi_sheet_tap(a: &mut App, id: i32, arg: i32) {
    let now = now_ms();
    match id {
        H_WIFI_RADIO => {
            let action = match arg {
                WIFI_ON => WifiAction::On,
                WIFI_OFF => WifiAction::Off,
                _ => WifiAction::Networks,
            };
            if action == WifiAction::On {
                let ws = &mut lib(a).wifi;
                ws.scan_auto = true;
                ws.view = WIFI_VIEW_NEARBY;
                ws.scroll = 0;
                ws.scroll_drag = None;
            } else if action == WifiAction::Off {
                lib(a).wifi.scan_auto = false;
            }
            wifi_request(action);
        }
        H_WIFI_TAB => {
            let ws = &mut lib(a).wifi;
            if arg == WIFI_VIEW_SAVED as i32 {
                ws.view = if ws.view == WIFI_VIEW_SAVED { WIFI_VIEW_NEARBY } else { WIFI_VIEW_SAVED };
                if ws.view == WIFI_VIEW_NEARBY && ws.near.is_empty() {
                    ws.scan_auto = true;
                }
            } else {
                ws.view = WIFI_VIEW_NEARBY;
            }
            ws.scroll = 0;
            ws.scroll_drag = None;
            ws.other = None;
        }
        H_WIFI_SCAN => {
            let ws = &mut lib(a).wifi;
            ws.view = WIFI_VIEW_NEARBY;
            ws.scroll = 0;
            ws.scroll_drag = None;
            ws.other = None;
            ws.scan_auto = false;
            if !wifi_scan() {
                // TRANSLATORS: (Settings > Wi-Fi) the toast when a scan cannot be asked for now.
                app_toast(a, tr("Could not scan for networks."), 3000);
                return;
            }
            ws.refresh_wait = true;
        }
        H_WIFI_OTHER => {
            // (the page under the keyboard is still the nearby one; the name makes the security page)
            {
                let ws = &mut lib(a).wifi;
                ws.other = None;
                ws.scroll = 0;
                ws.scroll_drag = None;
            }
            // TRANSLATORS: the keyboard's title for a hidden Wi-Fi network's name.
            open_kb(a, KbPurpose::WifiName, tr("Hidden network name"), b"");
            return;
        }
        H_WIFI_OTHER_OPEN => {
            let Some(n) = lib(a).wifi.other.clone() else { return };
            if !wifi_add(&n, b"", WifiSecurity::Open) {
                // TRANSLATORS: (Settings > Wi-Fi) the toast when a network profile could not be created.
                app_toast(a, tr("Could not add the network."), 3000);
                return;
            }
            let ws = &mut lib(a).wifi;
            ws.view = WIFI_VIEW_NEARBY;
            ws.other = None;
            ws.scroll = 0;
            ws.scroll_drag = None;
            ws.scan_auto = true;
            // TRANSLATORS: (Settings > Wi-Fi) the toast while a hidden network's profile is being created.
            app_toast(a, tr("Adding network…"), 2000);
        }
        H_WIFI_OTHER_WPA => {
            let Some(n) = lib(a).wifi.other.clone() else { return };
            wifi_password_start(a, n, WifiSecurity::Wpa2);
            return;
        }
        H_WIFI_OTHER_WPA3 => {
            let Some(n) = lib(a).wifi.other.clone() else { return };
            wifi_password_start(a, n, WifiSecurity::Wpa3);
            return;
        }
        H_WIFI_OTHER_EAP => {
            let Some(n) = lib(a).wifi.other.clone() else { return };
            wifi_eap_start(
                a,
                WifiNetwork {
                    essid: n,
                    security: WifiSecurity::Enterprise,
                    signal: 0,
                    signal_max: 1,
                    known: false,
                },
            );
        }
        H_WIFI_OTHER_CANCEL => {
            let ws = &mut lib(a).wifi;
            ws.view = WIFI_VIEW_NEARBY;
            ws.other = None;
            ws.scroll = 0;
            ws.scroll_drag = None;
        }
        H_WIFI_DELETE => {
            let ws = &lib(a).wifi;
            let Some(n) = ws.rows.get(arg as usize).cloned() else { return };
            if !wifi_delete(&n) {
                // TRANSLATORS: (Settings > Wi-Fi) the toast when a saved network could not be forgotten.
                app_toast(a, tr("Could not forget the network."), 3000);
                return;
            }
            // TRANSLATORS: (Settings > Wi-Fi) the toast while a saved network's profile is being deleted.
            app_toast(a, tr("Forgetting network…"), 2000);
        }
        H_WIFI_NET => {
            let ws = &lib(a).wifi;
            let Some(n) = ws.rows.get(arg as usize).cloned() else { return };
            // (the network connected to: tapping it again disconnects, keeping the radio on; the
            // Kindle may bracket the connected profile's name, so compare it unwrapped)
            if wifi_is_current(&n, ws.shown.as_ref().and_then(|w| w.current.as_deref())) {
                wifi_disconnect_tapped(a);
                return;
            }
            lib(a).wifi.reread = false;
            wifi_join(&n);
        }
        H_WIFI_NEAR => {
            let ws = &lib(a).wifi;
            let Some(n) = ws.near.get(arg as usize).cloned() else { return };
            if wifi_is_current(&n.essid, ws.shown.as_ref().and_then(|w| w.current.as_deref())) {
                wifi_disconnect_tapped(a);
                return;
            }
            if !n.known && !ws.shown.as_ref().is_some_and(|w| w.saved.as_ref().is_some_and(|s| s.iter().any(|x| x == &n.essid))) {
                match n.security {
                    WifiSecurity::Open => {
                        if !wifi_add(&n.essid, b"", WifiSecurity::Open) {
                            // TRANSLATORS: (Settings > Wi-Fi) the toast when a network profile could not be created.
                            app_toast(a, tr("Could not add the network."), 3000);
                            return;
                        }
                        // TRANSLATORS: (Settings > Wi-Fi) the toast while an open network's profile is being created.
                        app_toast(a, tr("Adding network…"), 2000);
                    }
                    WifiSecurity::Wpa | WifiSecurity::Wpa2 | WifiSecurity::Wpa3 => {
                        wifi_password_start(a, n.essid.clone(), n.security);
                        return;
                    }
                    WifiSecurity::Enterprise => {
                        wifi_eap_start(a, n.clone());
                        return;
                    }
                    _ => {
                        // TRANSLATORS: (Settings > Wi-Fi) a scanned network Grimoire cannot add yet.
                        app_toast(a, tr("This network's security is not supported yet."), 3500);
                        return;
                    }
                }
            } else {
                lib(a).wifi.reread = false;
                wifi_join(&n.essid);
            }
        }
        _ => return,
    }
    let ws = &mut lib(a).wifi;
    ws.check_at = now;
    ws.watch_at = now;
    // (a tap is activity: the next quiet refresh waits 20 s from it)
    ws.refresh_at = now;
    redraw(a, RefreshMode::Ui);
}

fn wifi_sheet_tick(a: &mut App, now: u32) {
    // A quiet scan's 20 s wait starts when its answer is in, not when it was asked for: a scan that
    // takes 20 s must not be followed by another one straight away.
    {
        let ws = &mut lib(a).wifi;
        if ws.refresh_wait && !wifi_scanning() {
            ws.refresh_wait = false;
            ws.refresh_at = now;
        }
    }
    if !wifi_busy() {
        let st = wifi_status();
        let join = wifi_joining().map(|j| j.1);
        let ws = &mut lib(a).wifi;
        if !ws.asked {
            ws.asked = wifi_request(WifiAction::Networks);
        }
        match st {
            // A switch the helper refused leaves the status unread: read everything once more. A failed read
            // waits for Retry.
            None => {
                if join == Some(WifiJoin::Failed) && !ws.reread {
                    ws.reread = true;
                    wifi_request(WifiAction::Networks);
                }
            }
            Some(st) => {
                let on = st.wireless && st.enabled;
                // A scan while a switch is settling, or while a supplicant-owned EAP/WPA3 link is
                // connected, can make the Kindle deauthenticate from the AP it just joined. A stale
                // marker with no link is safe to scan: the sheet must not look stuck on it either.
                let supplicant_connected = st.supplicant && st.connected == Some(true);
                let no_scan = join == Some(WifiJoin::Pending) || supplicant_connected;
                if !on {
                    ws.relisted = false;
                }
                if on && ws.scan_auto && supplicant_connected {
                    // The list cannot refresh without risking the link. Stop claiming a scan is under
                    // way, and force the sheet to draw the Rescan hint instead.
                    ws.scan_auto = false;
                    ws.shown = None;
                } else if on && ws.scan_auto && !wifi_scanning() {
                    ws.scan_auto = false;
                    ws.refresh_at = now;
                    ws.refresh_wait = wifi_scan();
                } else if on && wifi_saved().is_none() && !ws.relisted {
                    ws.relisted = true;
                    ws.check_at = now;
                    wifi_request(WifiAction::Networks);
                } else if on
                    && ws.view == WIFI_VIEW_NEARBY
                    && !wifi_scanning()
                    && !no_scan
                    && wifi_nearby().is_some()
                    && now.wrapping_sub(ws.refresh_at) >= WIFI_REFRESH_MS
                {
                    // Keep the list: only its changed names are merged in when the scan returns.
                    ws.refresh_at = now;
                    ws.refresh_wait = wifi_scan_quiet();
                } else if now.wrapping_sub(ws.check_at) > 2000
                    && (join == Some(WifiJoin::Pending)
                        || (on && st.connected != Some(true) && now.wrapping_sub(ws.watch_at) < 45000))
                {
                    ws.check_at = now;
                    wifi_request(WifiAction::Current);
                }
            }
        }
    }
    if lib(a).wifi.shown.as_ref() != Some(&wifi_shown()) {
        redraw(a, RefreshMode::Ui);
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
        std::fs::metadata(buttons_pads_path(a)).is_ok()
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
        let title = match buttons_profile(a).filter(|_| kind.modal == BUTTONS.modal) {
            // TRANSLATORS: the Buttons sheet's title while it learns one keyboard's or controller's own
            // buttons (Settings > Shortcuts, Own shortcuts for this device on); %s is the device's own name.
            Some(i) => cformat!(tr("Buttons: %s"), &a.shortcut_profiles[i].name),
            None => tr(kind.title).to_string(),
        };
        draw_settings_menu(a, &title, &rows);
    } else {
        draw_bind_group(a, kind, group as usize);
    }
}

fn draw_bind_group(a: &mut App, kind: BindKind, group: usize) {
    let n = binding_row_count(kind, group) as i32;
    let (aw, ah) = (a.w, a.h);
    let title = tr(ACTION_GROUPS[group].title);
    let page = settings_page(&mut a.screen, aw, ah, title, true, false);
    hits_add(&mut lib(a).hits, page.back, bind_done_hit(kind), 0);
    hits_add(&mut lib(a).hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    let gap = ui_px(14.0);
    let rh = ((in_.h - (n - 1).max(0) * gap) / n.max(1)).clamp(ui_px(90.0), ui_px(140.0));
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
    // (a submenu's rows are Settings' cards: the row being learned is filled, the rest outlined)
    settings_card(c, r, if selected { ButtonStyle::Selected } else { ButtonStyle::Plain });
    let fg = if selected { 255 } else { 0 };
    let pad = settings_pad();
    let mut text_x = r.x + pad;
    if row.icon != IconKind::None {
        let isz = rh - ui_px(30.0);
        ui_icon(c, row.icon, Rect::new(r.x + pad, r.y + ui_px(15.0), isz, isz), fg);
        text_x += isz + settings_gap();
    }
    let thumb_w = if thumb.is_some() { ui_px(120.0) } else { 0 };
    let text_w = r.x + r.w - text_x - pad - thumb_w;
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
    let bw = (in_.w - 3 * gap) / 4;
    let use_r = Rect::new(in_.x, by, bw, ui_px(110.0));
    let retry_r = Rect::new(in_.x + bw + gap, by, bw, ui_px(110.0));
    let disable_r = Rect::new(retry_r.x + bw + gap, by, bw, ui_px(110.0));
    let cancel_r = Rect::new(
        disable_r.x + bw + gap,
        by,
        in_.w - 3 * bw - 3 * gap,
        ui_px(110.0),
    );
    // TRANSLATORS: the record card's buttons; Use saves the recording, Try again clears this
    // attempt, Disable saves "no gesture" for the action, and Cancel returns to the Gestures sheet.
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
        disable_r,
        IconKind::Ban,
        // TRANSLATORS: the record card's Disable button (about 290 px at 34 px in capitals): it removes
        // every gesture for the action (the built-in ones too), so the row says "no gesture".
        tr("Disable"),
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
    hits_add(&mut s.hits, disable_r, H_RECORD_DISABLE, 0);
    hits_add(&mut s.hits, cancel_r, H_RECORD_CANCEL, 0);
}

// The language Settings > Language has picked: its index, or -1 for automatic (language=auto,
// or a language this version does not speak).
fn language_picked(a: &App) -> i32 {
    language_picked_settings(&a.settings)
}

fn language_picked_settings(settings: &Settings) -> i32 {
    if i18n_setting_known(settings.language.as_str()) {
        i18n_find(settings.language.as_str())
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

/// The one Settings layout, edited by variables: a full screen with the title (at the one title
/// size) above the rule, an optional Back button at the top left, an X icon alone at the top right,
/// the content below, and an optional strip at the foot for a page's own control. The main page and
/// every submenu are this same frame.
pub(crate) struct SettingsPage {
    pub content: Rect,
    pub back: Rect,
    pub close: Rect,
    /// The strip at the foot a page with a control there (Pen feel's Reset tool) leaves for it.
    pub footer: Rect,
}

pub(crate) fn settings_page(
    c: &mut Canvas,
    aw: i32,
    ah: i32,
    title: &str,
    back: bool,
    footer: bool,
) -> SettingsPage {
    canvas_fill(c, Rect::new(0, 0, aw, ah), 255);
    let pad = ui_px(50.0);
    let th = ui_px(190.0);
    let bh = ui_px(150.0);
    let mut back_r = RECT_EMPTY;
    let mut tx = pad;
    if back {
        back_r = Rect::new(pad, (th - bh) / 2, bh, bh);
        ui_button(c, back_r, IconKind::Back, "", ButtonStyle::Plain);
        tx = back_r.x + bh + ui_px(30.0);
    }
    // Every Settings page closes with this X alone at the top right (never a Done at the foot).
    let close_r = Rect::new(aw - pad - bh, (th - bh) / 2, bh, bh);
    ui_button(c, close_r, IconKind::Close, "", ButtonStyle::Plain);
    // (the same title face and size on the main page and on every submenu)
    font_draw_left(
        c,
        FontFace::Title,
        ui_title_size(),
        Rect::new(tx, 0, (close_r.x - tx - ui_px(30.0)).max(0), th),
        title,
        0,
    );
    ui_rule(c, 0, aw, th, ui_px(4.0), 0);
    let footer_h = if footer { ui_px(170.0) } else { ui_px(36.0) };
    let content = Rect::new(pad, th + ui_px(36.0), aw - 2 * pad, ah - th - ui_px(36.0) - footer_h);
    let footer_r = if footer {
        Rect::new(pad, ah - ui_px(30.0) - ui_px(120.0), aw - 2 * pad, ui_px(120.0))
    } else {
        RECT_EMPTY
    };
    SettingsPage { content, back: back_r, close: close_r, footer: footer_r }
}

/// Maps a settings row to the hit id its action already handles: adding a row in settingsmenu.rs
/// is one line here (and its handler in act if it is new) and the page layout picks it up.
fn setting_hit(id: settingsmenu::SettingId) -> i32 {
    use settingsmenu::SettingId::*;
    match id {
        Display => H_SET_DISPLAY,
        SleepSettings => H_SET_SLEEP,
        ClockSettings => H_SET_CLOCK,
        Theme => H_SET_THEME,
        DarkMode => H_SET_DARK,
        Font => H_SET_FONT,
        Rotate => H_SET_ROTATE,
        Pinch => H_SET_PINCH,
        PenFeel => H_SET_PENS,
        SideButton => H_SET_SIDE,
        EraserMode => H_SET_ERASER,
        Calibration => H_SET_CALIB,
        Shortcuts => H_SHORTCUTS,
        Pair => H_PAIR_MENU,
        Gestures => H_GESTURES,
        Language => H_SET_LANGUAGE,
        Mirror => H_SET_MIRROR,
        Wifi => H_WIFI_MENU,
        KOReader => H_KOREADER,
        Updates => H_UPDATES,
        AboutDevice => H_SET_DEVICE,
    }
}

/// Settings' page: a full screen, its categories down the left, the active one's settings to
/// their right, and the X at the top right. The categories, rows, labels and icons come from
/// settingsmenu.rs; this function only lays them out and maps each row to its action.
fn draw_settings(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let classic = ui_theme() == UiTheme::Classic;
    let App {
        screen: c,
        lib,
        settings,
        ..
    } = a;
    let s = lib.as_deref_mut().unwrap();
    let page = settings_page(c, aw, ah, tr("Settings"), false, false);
    // (a tap outside every row finds no hit and does nothing: never a hit over the whole page, whose
    // press feedback would invert the screen and refresh it whole)
    let pad = page.content.x;
    let top = page.content.y;
    let cats = &settingsmenu::CATEGORIES;
    let side_w = (aw * 30 / 100).clamp(ui_px(320.0), ui_px(560.0));
    let ncat = cats.len() as i32;
    let content_bottom = page.content.y + page.content.h;
    let cat_area = Rect::new(pad, top, side_w, content_bottom - top);
    // A list is a page of ui_row's (ui_row_page): the same card, icon, label and spacing whether
    // there are six categories or sixty, and its own page arrows when they no longer fit.
    let active = clampi(s.settings_cat, 0, ncat - 1);
    let mut cat_page = ui_row_page(cat_area, cats.len(), s.settings_cat_page, ui_px(120.0), ui_px(72.0));
    // (the open category is brought on screen when it changes: the page arrows stay free to page
    // the categories themselves)
    if s.settings_cat_shown != active {
        s.settings_cat_shown = active;
        if !cat_page.rows.is_empty()
            && !(cat_page.first..cat_page.first + cat_page.rows.len()).contains(&(active as usize))
        {
            let per = cat_page.rows.len().max(1);
            cat_page = ui_row_page(cat_area, cats.len(), (active as usize / per) as i32, ui_px(120.0), ui_px(72.0));
        }
    }
    s.settings_cat_page = cat_page.page;
    for (i, cat) in cats.iter().enumerate().skip(cat_page.first).take(cat_page.rows.len()) {
        let r = cat_page.rows[i - cat_page.first];
        let row = UiRow::new(cat.icon, tr(cat.title)).bold();
        let row = UiRow {
            style: if i as i32 == active { ButtonStyle::Selected } else { ButtonStyle::Plain },
            ..row
        };
        ui_row(c, r, &row);
        hits_add(&mut s.hits, r, H_SET_CAT, i as i32);
    }
    ui_row_pager(c, &cat_page);
    if cat_page.page > 0 {
        hits_add(&mut s.hits, cat_page.prev, H_SET_CAT_PAGE, -1);
    }
    if cat_page.page + 1 < cat_page.pages {
        hits_add(&mut s.hits, cat_page.next, H_SET_CAT_PAGE, 1);
    }
    let cat = &cats[active as usize];
    let content_x = pad + side_w + ui_px(48.0);
    let content_w = aw - content_x - pad;
    font_draw_left(
        c,
        FontFace::Bold,
        ui_text_size(),
        Rect::new(content_x, top, content_w, ui_px(70.0)),
        tr(cat.title),
        0,
    );
    let list_top = top + ui_px(86.0);
    // (what the platform does not leave to Grimoire has no row here: light.rs, Services)
    let services = sys_services();
    let shown: Vec<&settingsmenu::Item> = cat
        .items
        .iter()
        .filter(|it| it.show.visible(classic, services))
        .collect();
    let list_area = Rect::new(content_x, list_top, content_w, content_bottom - list_top - ui_px(20.0));
    let list = ui_row_page(list_area, shown.len(), s.settings_rows_page, ui_px(118.0), ui_px(90.0));
    s.settings_rows_page = list.page;
    let mut label_buf = String::new();
    let mut lang_buf = String::new();
    for (r, item) in list.rows.iter().zip(shown[list.first..list.first + list.rows.len()].iter()) {
        let label: &str = match item.label {
            settingsmenu::Label::Text(msgid) => tr(msgid),
            settingsmenu::Label::Display => {
                // (without a front light the row is called what its sheet is: no brightness or warmth to name)
                tr(if light_available() {
                    settingsmenu::DISPLAY_WITH_LIGHT
                } else {
                    settingsmenu::DISPLAY_NO_LIGHT
                })
            }
            settingsmenu::Label::Style => tr(if classic {
                settingsmenu::STYLE_CLASSIC
            } else {
                settingsmenu::STYLE_GOTHIC
            }),
            settingsmenu::Label::Font => {
                label_buf.clear();
                // (one of the owner's own fonts by its family's name, cut short as a name is)
                let own_name = crate::userfonts::userfonts_in_use_name();
                // TRANSLATORS: (Settings, Classic only) the row that opens the Font sheet, with the font in use
                // by its short name, never translated ("Terminess", "Atkinson", "Noto Sans", "Literata"; one of
                // your own fonts by its family's name, cut short if it is long).
                csnprintf!(&mut label_buf, 96, tr("Font: %s"), own_name.as_deref().unwrap_or(FONT_FAMILIES[font_family()].short));
                &label_buf
            }
            settingsmenu::Label::Language => {
                let picked = language_picked_settings(settings);
                lang_buf.clear();
                if picked < 0 {
                    language_auto_label(&mut lang_buf, 96);
                } else {
                    csnprintf!(
                        &mut lang_buf,
                        96,
                        "%s",
                        i18n_language_name(picked).unwrap_or("(null)")
                    );
                }
                label_buf.clear();
                // TRANSLATORS: a row of Settings, with the choice: "Automatic (English)", or a language's name.
                csnprintf!(&mut label_buf, 160, tr("Language: %s"), lang_buf.as_str());
                &label_buf
            }
            settingsmenu::Label::SideButton => {
                label_buf.clear();
                // TRANSLATORS: Settings' Pen category row that opens the pen button's choices, with the
                // choice in use ("Pen button: Erases").
                csnprintf!(
                    &mut label_buf,
                    160,
                    tr("Pen button: %s"),
                    tr(settingsmenu::SIDE_BUTTON_NAMES[clampi(settings.side_button, 0, 3) as usize])
                );
                &label_buf
            }
            settingsmenu::Label::Eraser => {
                label_buf.clear();
                // TRANSLATORS: Settings' Pen category row that opens the eraser's choices, with the one in
                // use ("Eraser: Whole strokes").
                csnprintf!(
                    &mut label_buf,
                    160,
                    tr("Eraser: %s"),
                    tr(settingsmenu::ERASER_NAMES[clampi(settings.eraser_mode, 0, 1) as usize])
                );
                &label_buf
            }
            settingsmenu::Label::Mirror => {
                label_buf.clear();
                // TRANSLATORS: %s is the folder on the Kindle's storage (seen over USB) that gets a PDF of
                // every notebook.
                csnprintf!(
                    &mut label_buf,
                    256,
                    tr("Mirror PDFs to %s (USB)"),
                    "grimoire/PDFs"
                );
                &label_buf
            }
        };
        // The icon, the label and any switch are ui_row's: one design for every row in Settings,
        // whether it opens a page or is a choice.
        let row = match item.check.value(settings) {
            Some(on) => UiRow::toggled(item.icon, label, on),
            None => UiRow::new(item.icon, label),
        };
        ui_row(c, *r, &row);
        hits_add(&mut s.hits, *r, setting_hit(item.id), 0);
    }
    ui_row_pager(c, &list);
    if list.page > 0 {
        hits_add(&mut s.hits, list.prev, H_SET_ROWS_PAGE, -1);
    }
    if list.page + 1 < list.pages {
        hits_add(&mut s.hits, list.next, H_SET_ROWS_PAGE, 1);
    }
    hits_add(&mut s.hits, page.close, H_CANCEL, 0);
}

/// The Font sheet's third sample line, the same in every language: digits, the punctuation Spanish
/// opens with, the angle quotes, and accented letters (Japanese comes from DotGothic16 in every font).
const FONT_SAMPLE_CHARS: &str = "0123456789  \u{bf}? \u{a1}! \u{ab}\u{bb}  \u{e1}\u{e9}\u{ed}\u{f3}\u{fa} \u{f1} \u{e7} \u{fc} \u{df} \u{20ac}";

/// What a row of the owner's own fonts says under the font's name: that it is being checked, why it cannot be
/// used, that some of the language's letters come from Terminess, or nothing.
fn font_row_note(f: &crate::userfonts::UserFont) -> String {
    use crate::userfonts::{FontWhy, UserFontState};
    let mut out = String::new();
    match &f.state {
        // TRANSLATORS: (Settings > Font, a row of Your fonts) the font file is being checked in the background
        // before Grimoire uses it. Each of these row notes: about 900 px at 34 px, one line.
        UserFontState::Checking => out.push_str(tr("Checking...")),
        UserFontState::Checked => {}
        UserFontState::Ready(facts, _) => {
            if facts.lacks >> i18n_find(i18n_current()).clamp(0, 15) & 1 == 1 {
                // TRANSLATORS: (a row of Your fonts) the font can be used, but lacks some letters of the
                // language Grimoire is in: those are drawn in Terminess (the built-in font's name, kept).
                out.push_str(tr("Some letters come from Terminess."));
            }
        }
        // TRANSLATORS: (a row of Your fonts, greyed) the file is no font Grimoire can read: damaged, empty,
        // or one whose letters draw nothing.
        UserFontState::No(FontWhy::Unreadable) => out.push_str(tr("Unreadable font.")),
        UserFontState::No(FontWhy::TooBig) => {
            // TRANSLATORS: (a row of Your fonts, greyed) the font file is bigger than Grimoire takes; %d is 24.
            csnprintf!(&mut out, 96, tr("Too big (over %d MB)."), (crate::fontcheck::FONT_FILE_MAX >> 20) as i32);
        }
        // TRANSLATORS: (a row of Your fonts, greyed) a symbol font, or one without the letters A to Z.
        UserFontState::No(FontWhy::NoLatin) => out.push_str(tr("No Latin letters.")),
        // TRANSLATORS: (a row of Your fonts, greyed) a web font (.woff, .woff2): Grimoire takes .ttf and .otf
        // files. The two endings are file kinds: keep them as they are.
        UserFontState::No(FontWhy::Format) => out.push_str(tr("Use .ttf or .otf.")),
        // TRANSLATORS: (a row of Your fonts, greyed) checking the font took longer than Grimoire allows.
        UserFontState::No(FontWhy::Slow) => out.push_str(tr("Took too long to read.")),
        // TRANSLATORS: (a row of Your fonts, greyed) the font Grimoire was set to is no longer in the folder.
        UserFontState::No(FontWhy::Missing) => out.push_str(tr("File missing.")),
    }
    out
}

/// The check's preview (black on white) drawn into r: at its left, centred up and down, cut at r's right;
/// on a highlighted row white on black.
fn draw_font_preview(c: &mut Canvas, r: Rect, p: &Canvas, on: bool) {
    let dy = r.y + (r.h - p.h) / 2;
    let area = rect_intersect(rect_intersect(Rect::new(r.x, dy, p.w.min(r.w), p.h), r), c.clip);
    for y in area.y..area.y + area.h {
        for x in area.x..area.x + area.w {
            let v = p.px[((y - dy) * p.stride + (x - r.x)) as usize];
            let d = &mut c.px[(y * c.stride + x) as usize];
            *d = if on { (*d).max(255 - v) } else { (*d).min(v) };
        }
    }
}

/// A row of the owner's own fonts: the font's name drawn in itself (the picture the check made: the UI never
/// opens the file to list it), under it the file's name and what is to be said of it, in Terminess. A file that
/// cannot be used is greyed, with its name alone.
fn draw_font_own_row(c: &mut Canvas, r: Rect, f: &mut crate::userfonts::UserFont, on: bool, applied: bool) {
    crate::userfonts::userfont_resolve(f);
    let usable = f.usable();
    let fg: u8 = if on { 255 } else if usable { 0 } else { 110 };
    settings_card(c, r, if on { ButtonStyle::Selected } else if usable { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    let mark = r.h - ui_px(30.0);
    let x = r.x + settings_pad();
    // (the check mark's room is the row's own while it has none)
    let w = r.x + r.w - x - settings_pad() - if applied { mark } else { 0 };
    let top = Rect::new(x, r.y + ui_px(8.0), w, r.h - ui_px(58.0));
    let bottom = Rect::new(x, r.y + r.h - ui_px(54.0), w, ui_px(46.0));
    let note = font_row_note(f);
    let file = f.rel.rsplit('/').next().unwrap_or("").to_string();
    let base = f.base().to_string();
    let drawn = match crate::userfonts::userfont_preview(f) {
        Some(p) => {
            draw_font_preview(c, top, p, on);
            true
        }
        None => false,
    };
    if !drawn {
        font_draw_left_name(c, FontFace::FixedBold, ui_text_size(), top, &base, fg);
    }
    let nw = if note.is_empty() { 0 } else { font_measure(FontFace::Fixed, ui_small_size(), &note) };
    if usable {
        // The file's own name, cut short as names are, then the note at the row's right. In a narrow row
        // (sideways, two abreast) the name keeps three tenths of the line and the note is set smaller to fit.
        let gap = if nw > 0 { ui_px(30.0) } else { 0 };
        let room = (w - nw - gap).max(w * 3 / 10).min(w);
        font_draw_left_name(c, FontFace::Fixed, ui_small_size(), Rect::new(x, bottom.y, room, bottom.h), &file, fg);
        if nw > 0 {
            let nr = (w - room - gap).max(0).min(nw);
            ui_text(c, FontFace::Fixed, ui_small_size(), ui_small_size() * 0.7, Rect::new(x + w - nr, bottom.y, nr, bottom.h), &note, 1, false, fg);
        }
    } else {
        font_draw_left(c, FontFace::Fixed, ui_small_size(), bottom, &note, fg);
    }
    if applied {
        ui_icon(c, IconKind::Check, Rect::new(r.x + r.w - ui_px(25.0) - mark, r.y + ui_px(15.0), mark, mark), fg);
    }
}

/// Settings > Font (2.1, Classic only): on top a sample of the font highlighted (its name at the title size,
/// a sentence at the text size, digits and accents at the small size, all in it), then Built in: every family
/// by its name, each drawn in itself; then Your fonts (2.1.0-beta.2): the files of grimoire/fonts, a page of
/// them at a time (or where to copy some); the one in use with a check; then Back and Done. A tap applies a
/// font at once, so the sheet's own words are in the font in use, unless that font is one of the owner's own:
/// then they are Terminess, so a font that went wrong can always be taken back.
fn draw_font_sheet(a: &mut App) {
    if font_own() {
        font_with_family(0, || draw_font_sheet_in(a, None));
    } else {
        let applied = font_family();
        draw_font_sheet_in(a, Some(applied));
    }
}

fn draw_font_sheet_in(a: &mut App, applied: Option<usize>) {
    let (aw, ah) = (a.w, a.h);
    let own_applied = a.settings.font_file.as_str().to_string();
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let pick = s.font_pick.min(FONT_FAMILIES.len() - 1);
    let n = FONT_FAMILIES.len();
    // Sideways the rows stand two abreast.
    let cols: usize = if aw > ah { 2 } else { 1 };
    let (sample_h, rh, gap, pad) = (ui_px(300.0), ui_px(130.0), ui_px(16.0), ui_px(24.0));
    // (the two sections' labels: Built in's, and Your fonts' with its page count)
    let (lh, lh2) = (ui_px(60.0), ui_px(80.0));
    // TRANSLATORS: the title of Settings > Font, the list of fonts Classic can be drawn in.
    let page = settings_page(c, aw, ah, tr("Font"), true, false);
    hits_add(&mut s.hits, page.back, H_FONT_CANCEL, 0);
    hits_add(&mut s.hits, page.close, H_FONT_DONE, 0);
    let in_ = page.content;
    let sample = Rect::new(in_.x, in_.y, in_.w, sample_h);
    ui_frame(c, sample, ui_px(16.0), 0, false, ui_px(3.0));
    let tx = sample.x + pad;
    let tw = sample.w - 2 * pad;
    // TRANSLATORS: the Font sheet's sample sentence, drawn in each font at 44 px, at most two lines of
    // about 1600 px: a sentence that uses many of your language's letters (English: a pangram; Spanish
    // could be "La cigüeña tocaba el saxofón detrás del palenque de paja.").
    let sentence = tr("The quick brown fox jumps over the lazy dog.");
    let own_pick = if s.font_file.is_empty() { None } else { s.fonts.iter().position(|f| f.rel == s.font_file) };
    let draw_sample = |c: &mut Canvas, name: &str, named: bool| {
        let title = Rect::new(tx, sample.y + ui_px(16.0), tw, ui_px(90.0));
        if named {
            font_draw_left_name(c, FontFace::Title, ui_title_size(), title, name, 0);
        } else {
            font_draw_left(c, FontFace::Title, ui_title_size(), title, name, 0);
        }
        font_draw_wrapped(c, FontFace::Regular, ui_text_size(), Rect::new(tx, sample.y + ui_px(106.0), tw, ui_px(124.0)),
            sentence, 2, ui_px(62.0), false, 0);
        font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(tx, sample.y + ui_px(230.0), tw, ui_px(56.0)), FONT_SAMPLE_CHARS, 0);
    };
    match own_pick {
        Some(i) => {
            // A font of the owner's own: drawn by the font itself when it is loaded (the one in use, or the
            // sample this sheet holds); else by the picture the check made of its name.
            crate::userfonts::userfont_resolve(&mut s.fonts[i]);
            let name = s.fonts[i].family();
            let rel = s.fonts[i].rel.clone();
            if crate::userfonts::userfonts_in_use(&rel) {
                s.font_sample_how = "own";
                font_with_own(false, || draw_sample(c, &name, true));
            } else if crate::userfonts::userfonts_sample_is(&rel) {
                s.font_sample_how = "own";
                font_with_own(true, || draw_sample(c, &name, true));
            } else {
                s.font_sample_how = "preview";
                let f = &mut s.fonts[i];
                let base = f.base().to_string();
                if let Some(p) = crate::userfonts::userfont_preview(f) {
                    draw_font_preview(c, Rect::new(tx, sample.y + ui_px(16.0), tw, ui_px(124.0)), p, false);
                }
                font_draw_left_name(c, FontFace::Fixed, ui_small_size(), Rect::new(tx, sample.y + ui_px(230.0), tw, ui_px(56.0)), &base, 0);
            }
        }
        None => {
            s.font_sample_how = "family";
            font_with_family(pick, || draw_sample(c, FONT_FAMILIES[pick].name, false));
        }
    }
    s.font_own_rows.clear();
    let built_rows = n.div_ceil(cols) as i32;
    let fixed = sample_h + lh + built_rows * (rh + gap) + lh2;
    let rows_max = ((in_.h - fixed + gap) / (rh + gap)).max(1) as usize;
    let per_page = rows_max * cols;
    let nown = s.fonts.len();
    s.font_per_page = per_page;
    if s.font_first >= nown {
        s.font_first = nown.saturating_sub(1) / per_page * per_page;
    }
    s.font_first -= s.font_first % per_page;
    let first = s.font_first;
    let shown = nown.saturating_sub(first).min(per_page);
    let cw = (in_.w - (cols as i32 - 1) * gap) / cols as i32;
    let cell = |i: usize, top: i32| Rect::new(in_.x + (i % cols) as i32 * (cw + gap), top + (i / cols) as i32 * (rh + gap), cw, rh);
    let mut y = sample.y + sample.h;
    // TRANSLATORS: (Settings > Font) the heading over the fonts that come with Grimoire. 34 px bold, about
    // 800 px.
    font_draw_left(c, FontFace::Bold, ui_small_size(), Rect::new(in_.x, y, in_.w, lh), tr("Built in"), 0);
    y += lh;
    for (i, f) in FONT_FAMILIES.iter().enumerate() {
        let r = cell(i, y);
        let on = own_pick.is_none() && i == pick;
        let fg: u8 = if on { 255 } else { 0 };
        settings_card(c, r, if on { ButtonStyle::Selected } else { ButtonStyle::Plain });
        let mark = rh - ui_px(30.0);
        font_with_family(i, || {
            font_draw_left(c, FontFace::Bold, ui_text_size(), Rect::new(r.x + settings_pad(), r.y, r.w - 2 * settings_pad() - mark, rh), f.name, fg);
        });
        if applied == Some(i) {
            ui_icon(c, IconKind::Check, Rect::new(r.x + r.w - ui_px(25.0) - mark, r.y + ui_px(15.0), mark, mark), fg);
        }
        hits_add(&mut s.hits, r, H_FONT_PICK, i as i32);
    }
    y += built_rows * (rh + gap);
    // The owner's own: the heading names the folder, with the pages' arrows at its right when the fonts
    // take more than one.
    let mut heading = String::new();
    // TRANSLATORS: (Settings > Font) the heading over the fonts the owner copied onto the Kindle; %s is
    // their folder there, grimoire/fonts. 34 px bold, about 800 px.
    csnprintf!(&mut heading, 160, tr("Your fonts (%s)"), "grimoire/fonts");
    let pages = nown.div_ceil(per_page).max(1);
    let hw = if pages > 1 { in_.w - ui_px(420.0) } else { in_.w };
    font_draw_left(c, FontFace::Bold, ui_small_size(), Rect::new(in_.x, y, hw, lh2), &heading, 0);
    if pages > 1 {
        let (bw, bh2) = (ui_px(130.0), lh2 - ui_px(10.0));
        let prev = Rect::new(in_.x + in_.w - ui_px(400.0), y + ui_px(4.0), bw, bh2);
        let next = Rect::new(in_.x + in_.w - bw, y + ui_px(4.0), bw, bh2);
        let page = first / per_page;
        ui_button(c, prev, IconKind::Prev, "", if page > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        ui_button(c, next, IconKind::Next, "", if page + 1 < pages { ButtonStyle::Plain } else { ButtonStyle::Disabled });
        let count = format!("{}/{}", page + 1, pages);
        font_draw_centered(c, FontFace::Fixed, ui_small_size(), Rect::new(prev.x + bw, prev.y, next.x - prev.x - bw, bh2), &count, 0);
        if page > 0 {
            hits_add(&mut s.hits, prev, H_FONT_PAGE, -1);
        }
        if page + 1 < pages {
            hits_add(&mut s.hits, next, H_FONT_PAGE, 1);
        }
    }
    y += lh2;
    if nown == 0 {
        let mut line = String::new();
        // TRANSLATORS: (Settings > Font, under Your fonts while the folder is empty) how to add a font: %s is
        // the folder on the Kindle's storage, grimoire/fonts. ".ttf" and ".otf" are file kinds: keep them.
        // Up to two lines of about 1600 px at 44 px, or at 34 px.
        csnprintf!(&mut line, 256, tr("Copy .ttf or .otf files into %s over USB."), "grimoire/fonts");
        ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect::new(in_.x, y, in_.w, rh), &line, 2, false, 0);
    }
    for k in 0..shown {
        let idx = first + k;
        let r = cell(k, y);
        let f = &mut s.fonts[idx];
        let on = own_pick == Some(idx);
        let applied_own = !own_applied.is_empty() && f.rel == own_applied && crate::userfonts::userfonts_in_use(&own_applied);
        draw_font_own_row(c, r, f, on, applied_own);
        if f.usable() {
            hits_add(&mut s.hits, r, H_FONT_OWN, idx as i32);
        }
        s.font_own_rows.push((idx, r));
    }
}

/// Applies what the Font sheet has highlighted now: a family (settings.ini's font=) or one of the owner's own
/// fonts (font=file:<its path>), saved. The caller draws again in it with a Full refresh, as a style change
/// does. A tap on a font calls this at once; X only closes the sheet. One of the owner's own fonts has its
/// Bold file found again for it (font_bold= is for a hand). True when what draws the text changed.
fn font_apply_pick(a: &mut App) -> bool {
    let (pick, own, picked) = (lib(a).font_pick, lib(a).font_file.clone(), lib(a).font_picked);
    let was_own = a.settings.font_file.as_str().to_string();
    // What draws now changes nothing: also not a font= line whose font cannot be used (a file that is gone, a
    // newer version's font), which stays as it is until another font is picked.
    let drawn_own = if crate::userfonts::userfonts_in_use(&was_own) { was_own.clone() } else { String::new() };
    // (a family picked while settings.ini asks for a font of the owner's own that does not draw now, a font a
    // crash kept out, one still being checked or one that is gone, takes that font= line away: the next start
    // must not try it again)
    let drops_file = picked && own.is_empty() && !was_own.is_empty();
    if !drops_file && own == drawn_own && (!own.is_empty() || pick == font_family()) {
        return false;
    }
    if own.is_empty() {
        a.settings.font = pick as i32;
        a.settings.font_file.clear();
        a.settings.font_bold.clear();
        font_set_family(pick);
    } else {
        a.settings.font = 0;
        if own != was_own {
            a.settings.font_bold.clear();
        }
        a.settings.font_file.set(&own);
    }
    crate::userfonts::userfonts_apply(a);
    if !own.is_empty() && !crate::userfonts::userfonts_in_use(&own) {
        // The font did not load (too little memory, a file that changed): the setting stays, Terminess
        // draws, and the owner is told.
        font_set_family(0);
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Done on a font of the
        // owner's own that could not be loaded just now (too little memory free, or its file changed).
        app_toast(a, tr("Couldn't use that font."), 3000);
    }
    httpd_restyle(ui_theme() == UiTheme::Gothic); // (the share pages, if Live view is on)
    settings_save(&a.settings);
    true
}

/// The Font sheet's staying notice while fonts are checked ("Checking 3 fonts..."), gone when none is.
fn font_notice(a: &mut App) {
    let n = crate::userfonts::userfonts_checking(&lib(a).fonts);
    if lib(a).modal != Modal::Font || n == 0 {
        app_notice_clear(a, NoticeKey::FontCheck);
        return;
    }
    let mut msg = String::new();
    // TRANSLATORS: the notice that stays above the Font sheet's buttons while the fonts copied into
    // grimoire/fonts are checked in the background, with how many are left. One line of about 1700 px at 44 px
    // bold.
    csnprintf!(&mut msg, 160, ng_("Checking %d font...", "Checking %d fonts...", n as i64), n as i32);
    app_notice(a, NoticeKey::FontCheck, &msg);
}

/// The Font sheet's list read again from grimoire/fonts (as it opens, and when a check left files that were
/// still being copied): the fonts not checked yet go to the background job. The font settings.ini names stays
/// listed when its file is gone ("File missing.").
fn font_list_scan(a: &mut App) {
    let dpi = a.dpi;
    let mut list = crate::userfonts::userfonts_scan(dpi);
    let chosen = a.settings.font_file.as_str().to_string();
    if !chosen.is_empty() && !list.iter().any(|f| f.rel == chosen) {
        list.push(crate::userfonts::userfont_missing(&chosen));
    }
    let s = lib(a);
    s.fonts = list;
    s.fonts_looked_ms = now_ms();
}

/// Leaves the Font sheet's own state behind (Done, Cancel, or the sheet closed with the Library): the sample
/// font, the list and the notice.
fn font_sheet_closed(a: &mut App) {
    crate::userfonts::userfonts_sample(a, None);
    let s = lib(a);
    s.font_picked = false;
    s.fonts = Vec::new();
    s.font_file.clear();
    s.font_own_rows.clear();
    app_notice_clear(a, NoticeKey::FontCheck);
}

/// While the Font sheet is up: the rows of fonts being checked fill in as their verdicts land (the cache is
/// looked at twice a second, a listing of one small folder), and the folder is read again when a check left
/// files for later.
fn font_sheet_tick(a: &mut App, now: u32) {
    if crate::userfonts::userfonts_look_again(now) {
        font_list_scan(a);
        font_notice(a);
        redraw(a, RefreshMode::Ui);
        return;
    }
    if now.wrapping_sub(lib(a).fonts_looked_ms) < 500 {
        return;
    }
    lib(a).fonts_looked_ms = now;
    if crate::userfonts::userfonts_refresh(&mut lib(a).fonts) > 0 {
        font_notice(a);
        redraw(a, RefreshMode::Ui);
    }
}

/// A font check ended (app.rs's poll_jobs): the Font sheet, if it is up, shows what it found at once; and when
/// the font settings.ini asks for was waiting for it and draws from now on (`changed`), the Library is drawn
/// again in it, with a Full refresh as a change of font makes.
pub fn library_fonts_checked(a: &mut App, changed: bool) {
    if a.lib.is_none() || a.scr != ScreenId::Library {
        return;
    }
    if lib(a).modal == Modal::Font {
        crate::userfonts::userfonts_refresh(&mut lib(a).fonts);
        font_notice(a);
    }
    if changed {
        httpd_restyle(ui_theme() == UiTheme::Gothic);
        redraw(a, RefreshMode::Full);
    } else if lib(a).modal == Modal::Font {
        redraw(a, RefreshMode::Ui);
    }
}

// The item MORE, MOVE and DELETE act on (S->target), if it is one.
fn target_item(s: &LibraryState) -> Option<Item> {
    // (never the Files card: it is no notebook or folder, and its menu acts on neither; a menu left open over a list
    // that changed under it must not read it as notebook 0)
    if s.target >= 0 && (s.target as usize) < s.items.len() && !s.items[s.target as usize].is_files {
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
        Modal::Font => draw_font_sheet(a),
        Modal::Wifi => draw_wifi_sheet(a),
        Modal::About => {
            let (aw, ah) = (a.w, a.h);
            let App { screen: c, lib, .. } = a;
            let s = lib.as_deref_mut().unwrap();
            let list = s.about_view == AboutView::List;
            let page = if list { s.about_list_page } else { s.about_page };
            let h = crate::about::about_draw(c, aw, ah, s.about_bat, s.about_view, s.about_note, page);
            if list {
                s.about_list_page = h.page;
            } else {
                s.about_page = h.page;
            }
            s.about_pages = h.pages;
            s.about_rows = h.rows.iter().map(|r| r.1).collect();
            s.about_marked = h.marked;
            s.about_signed = h.signed;
            s.about_card = (h.card_blocks, h.card_notes, h.card_cut);
            s.about_body_px = h.body_px;
            s.about_lead = h.lead;
            s.about_area = h.area;
            for (r, id) in [(h.close, H_ABOUT_CLOSE), (h.prev, H_ABOUT_PREV), (h.next, H_ABOUT_NEXT), (h.more, H_ABOUT_MORE), (h.back, H_ABOUT_BACK)] {
                if !rect_is_empty(r) {
                    hits_add(&mut s.hits, r, id, 0);
                }
            }
            for (r, i) in &h.rows {
                hits_add(&mut s.hits, *r, H_ABOUT_ROW, *i as i32);
            }
        }
        Modal::Updates => draw_updates(a),
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
            // TRANSLATORS: the title of the sheet that pairs a controller (Settings > Pair > Pair a controller).
            draw_settings_menu(a, tr("Pair a controller"), &rows);
        }
        Modal::Buttons => draw_bind_sheet(a, BUTTONS),
        Modal::Gestures => draw_gestures_modal(a),
        Modal::Shortcuts => {
            let mut last = String::new();
            if a.last_shortcut_code != 0 {
                // TRANSLATORS: (Shortcuts sheet) the last key ("key") or controller button ("pad") pressed:
                // its Linux evdev code and the modifiers held, for writing shortcuts.ini by hand.
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
            let (code, mods, gamepad, hid_changes) = (
                a.last_shortcut_code,
                a.last_shortcut_mods,
                a.last_shortcut_gamepad,
                a.hid_changes,
            );
            let s = lib(a);
            s.shown_shortcut_code = code;
            s.shown_shortcut_mods = mods;
            s.shown_shortcut_gamepad = gamepad;
            s.shown_hid_changes = hid_changes;
            let on = app_shortcuts_on(a);
            // The device: the one chosen, else the first listed (a connected one before a saved one); none while
            // shortcuts are off (the one chosen is kept for when they are on again).
            let dev = shortcut_dev_now(a);
            if let Some(d) = &dev {
                lib(a).shortcut_dev = d.slug.clone();
            }
            let more = shortcut_devices(a).len() > 1;
            let own = dev.as_ref().and_then(|d| a.shortcut_profiles.iter().find(|p| p.slug == d.slug));
            let own_on = own.is_some_and(|p| p.on);
            let mut dev_line = String::new();
            let mut own_line = String::new();
            match &dev {
                None if !on => {
                    // TRANSLATORS: (Shortcuts sheet) the device row while Enable shortcuts is off: no keyboard
                    // or controller is read then, so none is listed (about 1300 px at 44 px in bold).
                    csnprintf!(&mut dev_line, 160, "%s", tr("Devices are listed while shortcuts are on"));
                }
                None => {
                    csnprintf!(&mut dev_line, 160, "%s", tr("No controller or keyboard connected"));
                }
                Some(d) => {
                    let mut name = String::new();
                    if d.connected {
                        csnprintf!(&mut name, 120, "%s", &d.name);
                    } else {
                        // TRANSLATORS: (Shortcuts sheet) a device with its own shortcuts saved that is not
                        // connected now; %s is its own name.
                        csnprintf!(&mut name, 120, tr("%s, not connected"), &d.name);
                    }
                    if more {
                        // TRANSLATORS: (Shortcuts sheet) the keyboard or controller the next two rows are about,
                        // with others listed, which a tap goes through (those connected, then those with
                        // shortcuts saved); %s is its own name (about 1300 px at 44 px in bold with a short
                        // name).
                        csnprintf!(&mut dev_line, 160, tr("Device: %s (tap for the next)"), &name);
                    } else {
                        // TRANSLATORS: (Shortcuts sheet) the same with no other device listed.
                        csnprintf!(&mut dev_line, 160, tr("Device: %s"), &name);
                    }
                }
            }
            match own.filter(|p| p.on) {
                Some(p) => {
                    // TRANSLATORS: (Shortcuts sheet) the device's own shortcuts are on: how many it has, and
                    // how many of its file's lines were refused (grimoire/shortcuts/<device>.ini).
                    csnprintf!(&mut own_line, 160, tr("Own shortcuts (%d, %d refused)"), p.map.count, p.map.rejected);
                }
                None => {
                    // TRANSLATORS: (Shortcuts sheet) a tap gives the device shortcuts of its own, a copy of
                    // the default ones to start from, or uses the ones it had again (about 1300 px at 44 px
                    // in bold).
                    csnprintf!(&mut own_line, 160, "%s", tr("Own shortcuts for this device"));
                }
            }
            let mut reload = String::new();
            csnprintf!(
                &mut reload,
                160,
                // TRANSLATORS: (Shortcuts sheet) reads grimoire/shortcuts.ini (and controller.ini, and every
                // device's own) again: how many shortcuts the default ones hold, and how many of their
                // lines were refused.
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
                MenuRow {
                    name: dev.is_some(),
                    ..menu_row(
                        IconKind::Gamepad,
                        &dev_line,
                        if dev.is_some() { H_SHORTCUT_DEVICE } else { H_NONE },
                        0,
                        if dev.is_some() { ButtonStyle::Plain } else { ButtonStyle::Disabled },
                    )
                },
                menu_row(
                    if own_on { IconKind::Check } else { IconKind::None },
                    &own_line,
                    H_SHORTCUT_OWN,
                    0,
                    if dev.is_some() { ButtonStyle::Plain } else { ButtonStyle::Disabled },
                ),
                // TRANSLATORS: (Shortcuts sheet) opens the Buttons sheet: which controller button does what.
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
            // TRANSLATORS: the Shortcuts sheet's title (Settings > Shortcuts): a keyboard's or a controller's
            // keys as shortcuts, each device's own if it has them.
            draw_settings_menu(a, tr("Shortcuts"), &rows);
        }
        Modal::PairMenu => {
            let mut status = String::new();
            hid_status(a, &mut status, 160);
            let bt_changes = a.bt_changes;
            lib(a).shown_bt_changes = bt_changes;
            let bs = bt_state();
            let on = app_shortcuts_on(a);
            if !on {
                // TRANSLATORS: (Pair sheet) its status line while shortcuts are off, which the Bluetooth helper
                // serves alone (about 1100 px at 44 px in bold, or two lines at 34 px).
                csnprintf!(&mut status, 160, "%s", tr("Shortcuts are off: turn them on in Settings > Shortcuts."));
            } else if bs == BtState::Starting {
                // TRANSLATORS: (Pair sheet) its status line while the Bluetooth helper starts, and when it
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
            let present = bt_present();
            let bt_up = bs == BtState::Owned || bs == BtState::External;
            let rows = [
                // TRANSLATORS: (Pair sheet) starts the Bluetooth helper with Grimoire, for a Bluetooth
                // controller; without the helper unpacked in grimoire/bluetooth, the second.
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
                // TRANSLATORS: (Pair sheet) opens the next sheet: find a Bluetooth controller and pair it.
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
                menu_row(IconKind::Close, tr("Done"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            draw_settings_menu(a, tr("Pair"), &rows);
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
            draw_settings_menu(a, "KOReader", &rows);
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
                graph_omit_row(&lib(a).books[it.nb], H_MORE_GRAPH),
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
        Modal::FilesMore => {
            let rows = [
                menu_row(IconKind::FolderOpen, tr("Open"), H_FILES_OPEN, 0, ButtonStyle::Plain),
                // TRANSLATORS: the Files card's menu in the Library (1470 px at 44 px in bold): the card goes; the file
                // picker's "Show Files in Library" puts it back.
                menu_row(IconKind::Ban, tr("Hide from Library"), H_FILES_HIDE, 0, ButtonStyle::Plain),
                menu_row(IconKind::Close, tr("Cancel"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            draw_menu(a, tr("Files"), &rows);
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
                // TRANSLATORS: a folder's menu (see Rename folder): to another folder or the top level,
                // with everything in it.
                menu_row(IconKind::FolderMove, tr("Move folder"), H_FMORE_MOVE, 0, ButtonStyle::Plain),
                graph_omit_folder_row(it.folder.as_str()),
                // (2.2, issue #160: Grimoire opens at this folder from now on; on the folder that is home, at the
                // Library's top again)
                if home_of(a) == Home::Library(it.folder.as_str().to_string()) {
                    menu_row(IconKind::Library, tr("Reset home to Notebooks"), H_FMORE_HOME, 1, ButtonStyle::Plain)
                } else {
                    menu_row(IconKind::Home, tr("Set as home"), H_FMORE_HOME, 0, ButtonStyle::Plain)
                },
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
            // Where it is now: a notebook's folder, or the folder a folder is in.
            let curf: FixedStr<STORE_NAME_LEN> =
                if it.is_folder { FixedStr::from_str(store_folder_parent(it.folder.as_str())) } else { s.books[it.nb].folder };
            let dests = move_dests(s, &it);
            // TRANSLATORS: where a notebook can go: the Library itself, out of every folder (also in New
            // notebook: "In: Library (top level)").
            rows.push(menu_row(
                IconKind::Library,
                tr("Library (top level)"),
                H_MOVE_TOP,
                0,
                if !curf.is_empty() { ButtonStyle::Plain } else { ButtonStyle::Selected },
            ));
            let mut k = s.folder_move_page * 6;
            while k < dests.len() && k < (s.folder_move_page + 1) * 6 {
                let f = dests[k];
                let st = if curf != s.folders[f].as_str() { ButtonStyle::Plain } else { ButtonStyle::Selected };
                rows.push(menu_row(IconKind::Folder, &s.folders[f], H_MOVE_TO, f as i32, st));
                k += 1;
            }
            if s.folder_move_page > 0 {
                rows.push(menu_row(IconKind::Back, tr("Previous folders"), H_MOVE_PREV, 0, ButtonStyle::Plain));
            }
            if k < dests.len() {
                rows.push(menu_row(IconKind::None, tr("More folders"), H_MOVE_NEXT, 0, ButtonStyle::Plain));
            }
            rows.push(menu_row(IconKind::FolderPlus, tr("New folder..."), H_MOVE_NEW, 0, ButtonStyle::Plain));
            rows.push(menu_row(IconKind::Close, tr("Cancel"), H_CANCEL, 0, ButtonStyle::Plain));
            // TRANSLATORS: the title of the list of folders a notebook (or a folder) can be moved to.
            draw_menu(a, tr("Move to"), &rows);
        }
        Modal::Merge => {
            let s = lib(a);
            let dest = s.merge_dest;
            let both = store_folder_unique(dest.as_str());
            let mut title = String::new();
            let mut keep = String::new();
            // TRANSLATORS: the title of the dialog that opens when a folder is moved or renamed onto the
            // name of one that is there (%s: that folder's name, as the Library shows it).
            csnprintf!(&mut title, 256, tr("A folder named \"%s\" is already there."), &name_label(store_folder_leaf(dest.as_str())));
            // TRANSLATORS: the second answer (1470 px at 44 px in bold; %s: the name the folder would get,
            // the other one's with a number, "School 2"): the two folders stay apart.
            csnprintf!(&mut keep, 256, tr("Keep both (as \"%s\")"), &name_label(store_folder_leaf(both.as_deref().unwrap_or(dest.as_str()))));
            let rows = [
                // TRANSLATORS: the first answer (1470 px at 44 px in bold): what is in the folder that was
                // moved or renamed goes into the folder that is there, and that folder is gone.
                menu_row(IconKind::FolderMove, tr("Merge into it"), H_MERGE_INTO, 0, ButtonStyle::Primary),
                menu_row(IconKind::Copy, &keep, H_MERGE_BOTH, 0, if both.is_some() { ButtonStyle::Plain } else { ButtonStyle::Disabled }),
                menu_row(IconKind::Close, tr("Cancel"), H_CANCEL, 0, ButtonStyle::Plain),
            ];
            draw_menu(a, &title, &rows);
        }
        Modal::Exit => {
            // Esc at the Library's top level (4.2): Yes leaves as the Exit button does, No stays; the highlighted
            // one is Enter's (Yes at first), Left and Right move it.
            let no = lib(a).exit_no;
            let rows = [
                // TRANSLATORS: Exit Grimoire?'s answers (1470 px at 44 px in bold): leave Grimoire, or stay.
                menu_row(IconKind::Exit, tr("Yes"), H_EXIT_YES, 0, if no { ButtonStyle::Plain } else { ButtonStyle::Selected }),
                menu_row(IconKind::Close, tr("No"), H_EXIT_NO, 0, if no { ButtonStyle::Selected } else { ButtonStyle::Plain }),
            ];
            // TRANSLATORS: the question Esc asks at the Library's top level (a keyboard's), before Grimoire leaves.
            draw_menu(a, tr("Exit Grimoire?"), &rows);
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
            // Automatic, then every language by its own name; a pick applies at once. A full screen, as
            // Settings' submenus are: the page's chrome and the choices as cards in two columns.
            let cur = language_picked(a);
            let mut auto_label = String::new();
            language_auto_label(&mut auto_label, 96);
            let mut rows: Vec<(String, i32, bool)> = Vec::with_capacity(12);
            rows.push((auto_label, -1, cur < 0));
            let mut i = 0;
            while i < i18n_language_count() && rows.len() < 12 {
                rows.push((
                    i18n_language_name(i).unwrap_or("(null)").to_string(),
                    i,
                    cur == i,
                ));
                i += 1;
            }
            // TRANSLATORS: the title of the list of languages (Settings > Language).
            draw_pick_sheet(a, tr("Language"), &rows, H_LANG_PICK, H_LANG_BACK);
        }
        Modal::SideButton => {
            let cur = clampi(a.settings.side_button, 0, 3);
            let rows: Vec<(String, i32, bool)> = settingsmenu::SIDE_BUTTON_LABELS
                .iter()
                .enumerate()
                .map(|(i, label)| (tr(label).to_string(), i as i32, i as i32 == cur))
                .collect();
            // TRANSLATORS: the title of the list of what the pen's button does while writing (Settings > Pen).
            draw_pick_sheet(a, tr("Pen button"), &rows, H_SIDE_PICK, H_SIDE_BACK);
        }
        Modal::EraserMode => {
            let cur = clampi(a.settings.eraser_mode, 0, 1);
            let rows: Vec<(String, i32, bool)> = settingsmenu::ERASER_LABELS
                .iter()
                .enumerate()
                .map(|(i, label)| (tr(label).to_string(), i as i32, i as i32 == cur))
                .collect();
            // TRANSLATORS: the title of the list of what the eraser does (Settings > Pen).
            draw_pick_sheet(a, tr("Eraser"), &rows, H_ERASER_PICK, H_ERASER_BACK);
        }
        Modal::DeviceInfo => draw_device(a),
        Modal::LinkQr => draw_link(a),
        Modal::Kb => {
            // The page the keyboard was opened over stays on the panel, the keys over its lower half.
            let parent = lib(a).kb_parent;
            if parent != Modal::None && parent != Modal::Kb {
                lib(a).modal = parent;
                draw_modal(a);
                lib(a).modal = Modal::Kb;
                // (the keys take every contact: the page below is drawn, not tapped)
                hits_clear(&mut lib(a).hits);
            }
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
    if files_shown(a) {
        let mut b = std::mem::take(&mut lib(a).files);
        b.draw(a);
        lib(a).files = b;
        // (Exit Grimoire?, asked at a home inside Files, 2.2, issue #160: the question over Files as over the
        // Library's top, its two answers the only targets)
        if lib(a).modal == Modal::Exit {
            hits_clear(&mut lib(a).hits);
            draw_modal(a);
            a.toast_floor = toast_floor(a);
        }
        app_invalidate_all(a, m);
        return;
    }
    if graph_shown(a) {
        graph_draw(a);
        a.toast_floor = 0;
        app_invalidate_all(a, m);
        return;
    }
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

/// The device's name for About: the Kindle's own model (its device tree, else /proc/usid's model
/// code, its first 6 characters: never the serial number), or this computer's.
fn device_name() -> String {
    // (the reads are here for the Kindle build and are type-checked by every build; a computer has
    // neither file, so the host and the simulator show their own line)
    if cfg!(feature = "kindle") {
        if let Ok(raw) = std::fs::read("/proc/device-tree/model") {
            let name: String = raw.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
            let name = name.trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
        if let Ok(f) = std::fs::File::open("/proc/usid") {
            use std::io::Read;
            let mut b = Vec::with_capacity(6);
            let _ = f.take(6).read_to_end(&mut b);
            let code: String = b.iter().map(|&c| if c.is_ascii_alphanumeric() { c as char } else { '?' }).collect();
            if !code.is_empty() {
                return format!("Kindle ({code})");
            }
        }
        return tr("Kindle").to_string();
    }
    // TRANSLATORS: Settings > System > About, the device's name on a computer (the Kindle build names
    // the Kindle's own model).
    tr("This computer").to_string()
}

/// One of Settings > System > About's links: the name its row shows and the address its page shows
/// as a QR code. A link is one line in ABOUT_LINKS: About lists them, pages the list when it
/// outgrows the screen (ui_row_page), and opens each on its own page. Its row is named
/// about.link.N for scenarios, N its place here (from 0), so add new links at the end.
struct AboutLink {
    name: &'static str,
    url: &'static str,
}

/// The links About lists, in order. Names are msgids, translated as they are drawn; no other code
/// changes when one is added here.
static ABOUT_LINKS: &[AboutLink] = &[
    AboutLink { name: n_("Ko-fi"), url: "https://ko-fi.com/B0B7EF4TJ" },
    AboutLink { name: n_("Grimoire GitHub"), url: "https://github.com/ganten1998/grimoire-scribe" },
    AboutLink { name: n_("Nebuff's GitHub"), url: "https://github.com/nebuff" },
    AboutLink { name: n_("Ganten's GitHub"), url: "https://github.com/ganten1998" },
];

/// Settings > System > About: what this device is, its version, a bar of the storage used (the data
/// root's filesystem) with the free room under it, and the links. One text size for the lines under
/// the name, as the rest of Settings.
fn draw_device(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let page = settings_page(c, aw, ah, tr("About"), true, false);
    hits_add(&mut s.hits, page.back, H_DEVICE_BACK, 0);
    hits_add(&mut s.hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    let px = ui_text_size();
    let name = device_name();
    font_draw_left(c, FontFace::Bold, ui_title_size(), Rect::new(in_.x, in_.y, in_.w, ui_px(110.0)), &name, 0);
    let mut ver = String::new();
    csnprintf!(&mut ver, 96, tr("Grimoire %s"), GRIMOIRE_VERSION);
    font_draw_left(c, FontFace::Regular, px, Rect::new(in_.x, in_.y + ui_px(115.0), in_.w, ui_px(70.0)), &ver, 0);
    let mut y = in_.y + ui_px(260.0);
    match crate::pdfimport::storage_space() {
        Some((total, free)) => {
            let used = total.saturating_sub(free);
            let (used_s, total_s, free_s) = (
                crate::browser::size_label(used),
                crate::browser::size_label(total),
                crate::browser::size_label(free),
            );
            let mut used_line = String::new();
            csnprintf!(&mut used_line, 256, tr("Storage: %s of %s used"), used_s.as_str(), total_s.as_str());
            font_draw_left(c, FontFace::Regular, px, Rect::new(in_.x, y, in_.w, ui_px(70.0)), &used_line, 0);
            // The bar runs the page's full width; the fill inside it is rounded as the bar is, so
            // the progress ends in a pill rather than a square corner.
            let bar = Rect::new(in_.x, y + ui_px(95.0), in_.w, ui_px(72.0));
            ui_frame(c, bar, bar.h / 2, 0, false, ui_px(3.0));
            let inner = rect_inflate(bar, -ui_px(10.0));
            let filled = if total > 0 { (inner.w as u64 * used / total) as i32 } else { 0 };
            if filled > 0 {
                let fr = Rect::new(inner.x, inner.y, filled.min(inner.w), inner.h);
                ui_frame(c, fr, fr.h / 2, 0, true, 0);
            }
            let mut free_line = String::new();
            csnprintf!(&mut free_line, 160, tr("%s free"), free_s.as_str());
            font_draw_left(c, FontFace::Regular, px, Rect::new(in_.x, bar.y + bar.h + ui_px(26.0), in_.w, ui_px(70.0)), &free_line, 0);
            y = bar.y + bar.h + ui_px(102.0);
        }
        None => {
            // TRANSLATORS: Settings > System > About, where the storage bar would be when the system
            // cannot tell the storage's size.
            font_draw_left(c, FontFace::Regular, px, Rect::new(in_.x, y, in_.w, ui_px(70.0)), tr("Storage: not available"), 0);
            y += ui_px(102.0);
        }
    }
    // The links, as rows a tap opens on their own page with a QR code a phone scans. The list pages
    // when it outgrows the screen (ui_row_page), so a new link is one line in ABOUT_LINKS.
    y += ui_px(40.0);
    // TRANSLATORS: Settings > System > About, the heading over its links (Ko-fi, GitHub, ...).
    font_draw_left(c, FontFace::Bold, px, Rect::new(in_.x, y, in_.w, ui_px(70.0)), tr("Links"), 0);
    y += ui_px(90.0);
    let area = Rect::new(in_.x, y, in_.w, (in_.y + in_.h - y).max(0));
    let list = ui_row_page(area, ABOUT_LINKS.len(), s.device_links_page, ui_px(130.0), ui_px(90.0));
    s.device_links_page = list.page;
    for (r, i) in list.rows.iter().zip(list.first..list.first + list.rows.len()) {
        let link = &ABOUT_LINKS[i];
        ui_row(c, *r, &UiRow::new(IconKind::None, tr(link.name)));
        hits_add(&mut s.hits, *r, H_LINK, i as i32);
    }
    ui_row_pager(c, &list);
    if list.pages > 1 {
        hits_add(&mut s.hits, list.prev, H_LINK_PAGE, -1);
        hits_add(&mut s.hits, list.next, H_LINK_PAGE, 1);
    }
}

/// Settings > System > About's link: its name as the page's title, its address as a QR code with the
/// address under it.
fn draw_link(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let i = (s.device_link.max(0) as usize).min(ABOUT_LINKS.len().saturating_sub(1));
    let link = &ABOUT_LINKS[i];
    let page = settings_page(c, aw, ah, tr(link.name), true, false);
    hits_add(&mut s.hits, page.back, H_LINK_BACK, 0);
    hits_add(&mut s.hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    // TRANSLATORS: Settings > System > About > a link, above the link's QR code.
    font_draw_left(c, FontFace::Regular, ui_text_size(), Rect::new(in_.x, in_.y, in_.w, ui_px(70.0)), tr("Scan with a phone's camera"), 0);
    let top = in_.y + ui_px(110.0);
    let side = (in_.h - ui_px(300.0)).min(in_.w);
    let sym = crate::qr::ui_qr(c, Rect::new(in_.x + (in_.w - side) / 2, top, side, side), link.url, false);
    let ty = if rect_is_empty(sym) { top } else { sym.y + sym.h + ui_px(30.0) };
    ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect::new(in_.x, ty, in_.w, ui_px(90.0)), link.url, 2, true, 0);
}

/// A Settings submenu of choices, as Language's: the page's chrome, the choices as cards in two
/// columns (the chosen one filled), and a tap picking one. `rows` is (label, arg, chosen).
fn draw_pick_sheet(a: &mut App, title: &str, rows: &[(String, i32, bool)], pick: i32, back: i32) {
    let (aw, ah) = (a.w, a.h);
    let App { screen: c, lib, .. } = a;
    let s = lib.as_deref_mut().unwrap();
    let page = settings_page(c, aw, ah, title, true, false);
    hits_add(&mut s.hits, page.back, back, 0);
    hits_add(&mut s.hits, page.close, H_SETTINGS_CLOSE, 0);
    let in_ = page.content;
    let gap = ui_px(16.0);
    let n = rows.len() as i32;
    let per = (n + 1) / 2;
    let cw = (in_.w - gap) / 2;
    let rh = ((in_.h - (per - 1) * gap) / per.max(1)).clamp(ui_px(90.0), ui_px(130.0));
    for (k, (label, arg, selected)) in rows.iter().enumerate() {
        let col = k as i32 / per;
        let ri = k as i32 % per;
        let r = Rect::new(in_.x + col * (cw + gap), in_.y + ri * (rh + gap), cw, rh);
        settings_card(c, r, if *selected { ButtonStyle::Selected } else { ButtonStyle::Plain });
        let fg: u8 = if *selected { 255 } else { 0 };
        ui_text(
            c,
            FontFace::Regular,
            ui_text_size(),
            ui_small_size(),
            rect_inflate(r, -ui_px(16.0)),
            label,
            2,
            false,
            fg,
        );
        hits_add(&mut s.hits, r, pick, *arg);
    }
}

fn open_modal(a: &mut App, m: Modal) {
    lib(a).modal = m;
    redraw(a, RefreshMode::Ui);
}

/// What's new as the bat and the first start after an update open it: the card.
fn about_open(s: &mut LibraryState) {
    s.about_view = AboutView::Card;
    s.about_page = 0;
    s.about_list_page = 0;
    s.about_area = RECT_EMPTY;
}

/// What's new after a step on it (Read all, a version's notes, Back, a page, Esc back to the list or the
/// card): the screen drawn again, and the panel refreshed only where the card or sheet was and where it
/// is now, as the sheet refreshes (Ui): the Library around it has not changed. Opening and Dismiss
/// refresh the whole screen, as every sheet's do.
fn about_step(a: &mut App) {
    let was = lib(a).about_area;
    if rect_is_empty(was) || graph_shown(a) || lib(a).settings_return_editor {
        redraw(a, RefreshMode::Ui);
        return;
    }
    draw_base(a);
    hits_clear(&mut lib(a).hits);
    draw_modal(a);
    a.toast_floor = toast_floor(a);
    let now = lib(a).about_area;
    app_invalidate(a, rect_union(was, now), RefreshMode::Ui);
}

fn close_modal(a: &mut App) {
    if lib(a).settings_return_editor {
        if lib(a).modal==Modal::Settings {let changed=lib(a).settings_editor_fit!=a.settings.landscape_fit;lib(a).settings_return_editor=false;lib(a).modal=Modal::None;library_leave(a);if changed {editor_fit_changed(a,true);}crate::editor::editor_settings_background(a);app_invalidate_all(a,RefreshMode::Ui);}
        else {lib(a).modal=Modal::Settings;redraw(a,RefreshMode::Ui);}
        return;
    }
    let was = std::mem::replace(&mut lib(a).modal, Modal::None);
    // (What's new was over the Library at a start whose home is inside Files, 2.2, issue #160: home now)
    if was == Modal::About && std::mem::take(&mut lib(a).home_deferred) {
        home_go(a);
        redraw(a, RefreshMode::Full);
        input_touch_context(library_touch_view(a));
        return;
    }
    redraw(a, RefreshMode::Ui);
}

/// The X on any Settings page, or on a screen Settings opened: the whole of Settings closes, back
/// to the Library—or to the editor it was opened over. Back still goes one step back.
fn close_settings(a: &mut App, mode: RefreshMode) {
    if lib(a).settings_return_editor {
        let changed = lib(a).settings_editor_fit != a.settings.landscape_fit;
        lib(a).settings_return_editor = false;
        lib(a).modal = Modal::None;
        library_leave(a);
        if changed {
            editor_fit_changed(a, true);
        }
        crate::editor::editor_settings_background(a);
        app_invalidate_all(a, mode);
        return;
    }
    lib(a).modal = Modal::None;
    redraw(a, mode);
}

/// Pen feel's X: Settings must not reopen behind the screen it came from (its Back still does).
pub(crate) fn settings_skip_reopen(a: &mut App) {
    if let Some(s) = a.lib.as_deref_mut() {
        s.reopen_settings = false;
    }
}

fn open_kb(a: &mut App, p: KbPurpose, title: &str, initial: &[u8]) {
    let s = lib(a);
    if s.modal != Modal::Kb {
        s.kb_parent = s.modal;
    }
    s.kbp = p;
    keyboard_open(&mut s.kb, title, initial);
    s.modal = Modal::Kb;
    redraw(a, RefreshMode::Ui);
}

fn open_kb_secret(a: &mut App, p: KbPurpose, title: &str) {
    let s = lib(a);
    if s.modal != Modal::Kb {
        s.kb_parent = s.modal;
    }
    s.kbp = p;
    keyboard_open_secret(&mut s.kb, title);
    s.modal = Modal::Kb;
    redraw(a, RefreshMode::Ui);
}

/// Whether a sheet is one of Settings' pages (the keyboard goes back to it, drawn unchanged).
fn settings_sheet(m: Modal) -> bool {
    matches!(
        m,
        Modal::Settings
            | Modal::Display
            | Modal::Language
            | Modal::SideButton
            | Modal::EraserMode
            | Modal::DeviceInfo
            | Modal::LinkQr
            | Modal::Updates
            | Modal::Shortcuts
            | Modal::PairMenu
            | Modal::Pair
            | Modal::Buttons
            | Modal::Gestures
            | Modal::Font
            | Modal::Wifi
    )
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
                tr("Couldn't use template (see %s)."),
                "grimoire.log"
            );
            app_toast(a, &msg, 4000);
        }
        return;
    }
    drop(nb);
    app_toast(a, tr("Couldn't create notebook (storage full?)"), 3000);
}

fn kb_finished(a: &mut App, ok: bool) {
    let s = lib(a);
    let txt = s.kb.text.as_str().to_string();
    let it = target_item(s);
    let kbp = s.kbp;
    if kbp == KbPurpose::WifiName {
        let parent = if s.kb_parent == Modal::Kb { Modal::Wifi } else { s.kb_parent };
        let name = s.kb.text.as_bytes().to_vec();
        if ok {
            if name.is_empty() || name.len() > 32 {
                // TRANSLATORS: (Settings > Wi-Fi > Other...) the hidden network name is empty or longer
                // than a Wi-Fi name can be.
                app_toast(a, tr("Use a network name of 1 to 32 characters."), 3500);
            } else {
                let s = lib(a);
                s.wifi.other = Some(name);
                s.wifi.view = WIFI_VIEW_OTHER;
                s.wifi.scroll = 0;
                s.wifi.scroll_drag = None;
            }
        }
        lib(a).modal = parent;
        redraw(a, RefreshMode::Ui);
        return;
    }
    if kbp == KbPurpose::WifiEapUser {
        let user = s.kb.text.as_bytes().to_vec();
        let parent = if s.kb_parent == Modal::Kb { Modal::Wifi } else { s.kb_parent };
        s.kb.text.clear();
        s.kb.caret = 0;
        s.kb.anchor = 0;
        let valid = ok && !user.is_empty() && user.len() <= 128 && !user.iter().any(|&b| b < 0x20 || b == 0x7f);
        if !valid {
            let mut user = user;
            user.fill(0);
            if ok {
                // TRANSLATORS: (Settings > Wi-Fi) the enterprise username is empty, a control
                // character, or longer than 128 bytes.
                app_toast(a, tr("Use a username of 1 to 128 characters."), 3500);
            }
            lib(a).modal = parent;
            redraw(a, RefreshMode::Ui);
            return;
        }
        let title = {
            let mut t = String::new();
            if let Some(n) = s.wifi.add.as_ref() {
                let shown = String::from_utf8_lossy(&n.essid);
                // TRANSLATORS: the password keyboard's title for a school/enterprise Wi-Fi network;
                // %s is its name.
                csnprintf!(&mut t, 200, tr("Password for %s"), shown.as_ref());
            }
            t
        };
        s.wifi.eap_user = Some(user);
        open_kb_secret(a, KbPurpose::WifiEapPassword, &title);
        return;
    }
    if kbp == KbPurpose::WifiPassword {
        let pending = s.wifi.add.take();
        let mut pass = s.kb.text.as_bytes().to_vec();
        // (the password lives only in that Vec now; the keyboard's bytes go too)
        s.kb.text.clear();
        s.kb.caret = 0;
        s.kb.anchor = 0;
        if ok {
            if let Some(n) = pending {
                if !wifi_add(&n.essid, &pass, n.security) {
                    app_toast(a, tr("Could not add the network."), 3000);
                }
            }
        }
        pass.fill(0);
        wifi_sheet_resume(a);
        return;
    }
    if kbp == KbPurpose::WifiEapPassword {
        let pending = s.wifi.add.take();
        let mut user = s.wifi.eap_user.take();
        let mut pass = s.kb.text.as_bytes().to_vec();
        s.kb.text.clear();
        s.kb.caret = 0;
        s.kb.anchor = 0;
        if ok {
            if let (Some(n), Some(u)) = (pending.as_ref(), user.as_ref()) {
                if !wifi_add_eap(&n.essid, u, &pass) {
                    app_toast(a, tr("Could not add the network."), 3000);
                }
            }
        }
        pass.fill(0);
        if let Some(mut u) = user.take() {
            u.fill(0);
        }
        wifi_sheet_resume(a);
        return;
    }
    let parent = s.kb_parent;
    let mut back = if settings_sheet(parent) { parent } else { Modal::None };
    if ok && !txt.is_empty() {
        match kbp {
            KbPurpose::NewName => {
                // (confirming the name the sheet offered is no name typed: a picked file then names its notebook, D28)
                let s = lib(a);
                if s.new_name_typed || txt != s.new_name.as_str() {
                    s.new_name.set(&txt);
                    s.new_name_typed = true;
                }
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
                    app_toast(a, tr("Couldn't create folder. Use a shorter name without a slash."), 3500);
                } else if kbp == KbPurpose::MoveNewFolder && it.is_some_and(|it| it.is_folder) {
                    if move_folder(a, &it.unwrap(), path.as_str()) {
                        back = Modal::Merge;
                    }
                } else if kbp == KbPurpose::MoveNewFolder {
                    if let Some(it) = it.filter(|it| !it.is_folder) {
                        let id = lib(a).books[it.nb].id.as_str().to_string();
                        if store_set_folder(&id, path.as_str()) == 0 {
                            mirror_follow(a, &id, false);
                        } else {
                            app_toast(a, tr("Couldn't move notebook."), 3000);
                        }
                    }
                }
            }
            KbPurpose::WifiPassword | KbPurpose::WifiName | KbPurpose::WifiEapUser | KbPurpose::WifiEapPassword => {}
            KbPurpose::RenameFolder => {
                if let Some(it) = it.filter(|it| it.is_folder) {
                    let mut dest = FixedStr::new();
                    let joined = store_folder_join(store_folder_parent(it.folder.as_str()), &txt, &mut dest);
                    if joined && merge_asked(a, it.folder.as_str(), dest.as_str()) {
                        back = Modal::Merge;
                    } else if !joined || store_rename_folder(it.folder.as_str(), dest.as_str()) != 0 {
                        app_toast(a, tr("Couldn't rename folder. Check name and destination."), 3500);
                    } else {
                        folder_follow(a, it.folder.as_str());
                        home_follow(a, it.folder.as_str(), dest.as_str());
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

/// Move to's folders for it (indices into s.folders): every folder for a notebook; for a folder,
/// every folder but itself and those inside it (store_rename_folder refuses those).
fn move_dests(s: &LibraryState, it: &Item) -> Vec<usize> {
    (0..s.folders.len())
        .filter(|&f| !it.is_folder || !store_folder_contains(it.folder.as_str(), s.folders[f].as_str()))
        .collect()
}

/// A folder, with all it holds, into parent ("" the top level): its name stays, its notebooks' mirror
/// PDFs follow. When parent has a folder of that name the Merge dialog opens instead (true); a move that
/// cannot be made at all is a toast.
fn move_folder(a: &mut App, it: &Item, parent: &str) -> bool {
    let old = it.folder.as_str().to_string();
    if store_folder_parent(&old) == parent {
        return false;
    }
    let mut dest = FixedStr::new();
    if store_folder_join(parent, store_folder_leaf(&old), &mut dest) && merge_asked(a, &old, dest.as_str()) {
        return true;
    }
    if !store_folder_join(parent, store_folder_leaf(&old), &mut dest) || store_rename_folder(&old, dest.as_str()) != 0 {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a folder could not be moved
        // (a name too long, or storage that failed); a folder of that name there opens a dialog instead.
        app_toast(a, tr("Couldn't move folder. Check the destination."), 3500);
        return false;
    }
    folder_follow(a, &old);
    home_follow(a, &old, dest.as_str());
    false
}

/// Folder old is to take the path dest: when a folder is there by that name (and dest is not inside old),
/// the Merge dialog is what the caller opens (true; its state is set). False: the path is free, or cannot
/// be asked about (the plain rename and its refusals follow).
fn merge_asked(a: &mut App, old: &str, dest: &str) -> bool {
    if old == dest || store_folder_contains(old, dest) || !store_folder_exists(dest) {
        return false;
    }
    let s = lib(a);
    s.merge_old.set(old);
    s.merge_dest.set(dest);
    true
}

/// The Merge dialog's answer: into the folder that is there, or beside it as "Name 2".
fn merge_answer(a: &mut App, both: bool) {
    let s = lib(a);
    let (old, dest) = (s.merge_old.as_str().to_string(), s.merge_dest.as_str().to_string());
    // (where the folder is afterwards: beside the one that is there under its new name, or in it)
    let to = if both { store_folder_unique(&dest) } else { Some(dest.clone()) };
    let rc = match &to {
        Some(path) if both => store_rename_folder(&old, path),
        Some(_) => store_merge_folder(&old, &dest),
        None => -1,
    };
    lib(a).modal = Modal::None;
    if rc == 0 {
        folder_follow(a, &old);
        home_follow(a, &old, to.as_deref().unwrap_or(&dest));
    } else {
        // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): Merge into it or Keep both
        // could not be done; nothing was changed.
        app_toast(a, tr("Couldn't merge the folders. Nothing was changed."), 3500);
    }
    reload(a);
    redraw(a, RefreshMode::Ui);
}

/// After the folder old was renamed or moved (the lists still from before): the mirror PDFs of the
/// notebooks in it and in the folders inside it follow, and the mirror directories those folders
/// leave behind go if empty.
fn folder_follow(a: &mut App, old: &str) {
    let s = lib(a);
    let ids: Vec<String> =
        s.books.iter().filter(|nb| store_folder_contains(old, nb.folder.as_str())).map(|nb| nb.id.as_str().to_string()).collect();
    let gone: Vec<String> = s.folders.iter().filter(|f| store_folder_contains(old, f)).cloned().collect();
    for id in ids {
        mirror_follow(a, &id, false);
    }
    let mirror_dir = app_mirror_dir();
    for f in gone {
        export_mirror_prune_folder(&f, &mirror_dir);
    }
}

/// After the folder old was renamed, moved or merged, and is `new` now: a home that was that folder, or a folder
/// inside it, is that folder still, under its new path (2.2, issue #160), so Grimoire opens there as before. A
/// path home cannot keep leaves home as it was: it is found gone the next time Grimoire goes there.
fn home_follow(a: &mut App, old: &str, new: &str) {
    let Home::Library(f) = home_of(a) else { return };
    if !store_folder_contains(old, &f) {
        return;
    }
    let moved = Home::Library(format!("{new}{}", &f[old.len()..]));
    if Home::parse(moved.value().as_bytes()) == Some(moved.clone()) {
        home_save(a, moved);
    }
}

fn act(a: &mut App, h: &Hit) {
    if lib(a).share_wifi_return && matches!(h.id, H_SETTINGS_BACK | H_SETTINGS_CLOSE) {
        lib(a).share_wifi_return = false;
        app_switch(a, ScreenId::Share);
        return;
    }
    // The page arrows are the only taps that keep a submenu on its page: any other tap opens, closes
    // or changes the list, which starts again at its first page (a longer list drawn by ui_row_page).
    if !matches!(h.id, H_SETTINGS_PAGE | H_SET_CAT_PAGE | H_SET_ROWS_PAGE) {
        lib(a).settings_menu_page = 0;
    }
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
            s.new_name_typed = false;
            s.new_paper = PAPER_NARROW_NM;
            s.new_tpl.clear();
            canvas_free(&mut s.tpl_prev);
            open_modal(a, Modal::New);
        }
        // TRANSLATORS: the keyboard's label for the name being typed: a folder's, or a notebook's.
        H_NEWFOLDER => open_kb(a, KbPurpose::NewFolder, c_("keyboard", "Folder"), b""),
        H_SHARE => share_open(a),
        H_SETTINGS => {
            let s = lib(a);
            s.settings_cat_page = 0;
            s.settings_cat_shown = -1;
            s.settings_rows_page = 0;
            s.settings_menu_page = 0;
            open_modal(a, Modal::Settings);
        }
        H_SET_CAT => {
            let s = lib(a);
            s.settings_cat = clampi(h.arg, 0, settingsmenu::CATEGORIES.len() as i32 - 1);
            s.settings_rows_page = 0;
            redraw(a, RefreshMode::Ui);
        }
        H_SET_CAT_PAGE | H_SET_ROWS_PAGE | H_SETTINGS_PAGE => {
            let s = lib(a);
            let page = if h.id == H_SET_CAT_PAGE {
                &mut s.settings_cat_page
            } else if h.id == H_SET_ROWS_PAGE {
                &mut s.settings_rows_page
            } else {
                &mut s.settings_menu_page
            };
            *page += h.arg;
            redraw(a, RefreshMode::Ui);
        }
        H_EXIT => a.running = false,
        H_EXIT_YES => {
            lib(a).modal = Modal::None;
            a.running = false;
        }
        H_EXIT_NO => close_modal(a),
        H_ABOUT => {
            about_open(lib(a));
            open_modal(a, Modal::About);
        }
        H_ABOUT_CLOSE => close_modal(a),
        H_ABOUT_PREV | H_ABOUT_NEXT => {
            // (the page shown's neighbour, when there is one: a greyed arrow does nothing)
            let s = lib(a);
            let page = if s.about_view == AboutView::List { &mut s.about_list_page } else { &mut s.about_page };
            let to = *page + if h.id == H_ABOUT_NEXT { 1 } else { -1 };
            if to >= 0 && to < s.about_pages {
                *page = to;
                about_step(a);
            }
        }
        // Read all: the list of every version's notes, from its first page.
        H_ABOUT_MORE => {
            let s = lib(a);
            s.about_view = AboutView::List;
            s.about_list_page = 0;
            about_step(a);
        }
        // A row of the list: that version's notes, from their first page.
        H_ABOUT_ROW => {
            let s = lib(a);
            s.about_view = AboutView::Note;
            s.about_note = h.arg.max(0) as usize;
            s.about_page = 0;
            about_step(a);
        }
        // Back from a version's notes: the list, at the page it was left at.
        H_ABOUT_BACK => {
            lib(a).about_view = AboutView::List;
            about_step(a);
        }
        H_PREV | H_NEXT => turn(a, h.id == H_NEXT),
        H_CARD => {
            let s = lib(a);
            if h.arg >= s.items.len() as i32 {
                return;
            }
            let item = s.items[h.arg as usize];
            if item.is_files {
                files_open_place(a);
            } else if item.is_folder {
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
            let Some(&item) = s.items.get(h.arg.max(0) as usize) else { return };
            s.target = h.arg;
            let m = if item.is_files {
                Modal::FilesMore
            } else if item.is_folder {
                Modal::FolderMore
            } else {
                Modal::More
            };
            open_modal(a, m);
        }
        H_FILES_OPEN => {
            lib(a).modal = Modal::None;
            files_open_place(a);
        }
        H_FILES_HIDE => {
            a.settings.files_card = false;
            settings_save(&a.settings);
            lib(a).modal = Modal::None;
            reload(a);
            redraw(a, RefreshMode::Ui);
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
        H_NEW_FILE => files_open(a),
        H_CANCEL => close_modal(a),
        H_SETTINGS_CLOSE => close_settings(a, RefreshMode::Ui),
        // Back (and Done) on a Settings submenu: to the Settings page it was opened from, or one step
        // back through Shortcuts / Pair, as Esc already does. It must never close Settings itself.
        H_SETTINGS_BACK => {
            let back = match lib(a).modal {
                Modal::Pair => Modal::PairMenu,
                Modal::Buttons => Modal::Shortcuts,
                _ => Modal::Settings,
            };
            open_modal(a, back);
        }
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
        H_RECORD_DISABLE => disable_recording(a),
        H_RECORD_CANCEL => close_record_card(a),
        H_SHORTCUTS => open_modal(a, Modal::Shortcuts),
        H_PAIR_MENU => open_modal(a, Modal::PairMenu),
        H_WIFI_MENU => wifi_sheet_open(a),
        H_WIFI_RADIO | H_WIFI_NET | H_WIFI_TAB | H_WIFI_SCAN | H_WIFI_NEAR | H_WIFI_DELETE
        | H_WIFI_OTHER | H_WIFI_OTHER_OPEN | H_WIFI_OTHER_WPA | H_WIFI_OTHER_WPA3 | H_WIFI_OTHER_EAP
        | H_WIFI_OTHER_CANCEL => {
            wifi_sheet_tap(a, h.id, h.arg)
        }
        H_SHORTCUT_DEVICE => {
            // The next device listed (the first after the last), whose own shortcuts the next row is about.
            let devs = shortcut_devices(a);
            if let Some(now) = shortcut_dev_now(a) {
                let i = devs.iter().position(|d| d.slug == now.slug).unwrap_or(0);
                lib(a).shortcut_dev = devs[(i + 1) % devs.len()].slug.clone();
            }
            redraw(a, RefreshMode::Ui);
        }
        H_SHORTCUT_OWN => toggle_own_shortcuts(a),
        H_SHORTCUT_ENABLE | H_SHORTCUT_RELOAD => {
            let mut path = String::new();
            csnprintf!(&mut path, 1024, "%s/shortcuts.ini", &store_root());
            if h.id == H_SHORTCUT_ENABLE {
                if !a.settings.shortcuts_enabled && shortcuts_write_defaults(&path) != 0 {
                    app_toast(a, tr("Couldn't save shortcuts.ini"), 5000);
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
            let dev = buttons_dev_for(a);
            let s = lib(a);
            s.buttons_dev = dev;
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
            let cp = buttons_pads_path(a);
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
            // (the Pair a controller sheet's Done goes back to Pair, the Buttons sheet's to Shortcuts)
            let back = if s.modal == Modal::Pair { Modal::PairMenu } else { Modal::Shortcuts };
            open_modal(a, back);
        }
        H_UPDATES => {
            // Opening the page runs the check (one already under way finishes first).
            if lib(a).upd != UPD_CHECKING {
                lib(a).upd = UPD_IDLE;
                if update_check(upd_preview(a)) {
                    lib(a).upd = UPD_CHECKING;
                }
            }
            open_modal(a, Modal::Updates);
        }
        H_UPD_CHANNEL => {
            let preview = !upd_preview(a);
            a.settings.updates = if preview { 2 } else { 1 };
            settings_save(&a.settings);
            // What the last check found was the other branch's: ask this one now (a check already
            // under way is left to finish).
            if !update_checking() {
                lib(a).upd = UPD_IDLE;
                if update_check(preview) {
                    lib(a).upd = UPD_CHECKING;
                }
            }
            redraw(a, RefreshMode::Ui);
        }
        H_UPD_NOW => {
            if upd_newer(lib(a)) {
                if update_request(upd_preview(a)) != 0 {
                    // TRANSLATORS: a toast: the update could not be asked for (the storage is full, say).
                    app_toast(a, tr("Couldn't start update"), 3000);
                    return;
                }
                qlog!("update: leaving so KPM can install %s", &lib(a).upd_latest);
                app_leave_for_update(a);
                return;
            }
            // Nothing newer (a failed or idle check): the button asks again.
            let preview = upd_preview(a);
            if update_check(preview) {
                lib(a).upd = UPD_CHECKING;
            }
            redraw(a, RefreshMode::Ui);
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
                    tr("Couldn't update KOReader. Patch conflict or storage error?"),
                    7000,
                );
            }
            redraw(a, RefreshMode::Ui);
        }
        H_SET_PENS => {
            let from_settings = lib(a).modal == Modal::Settings && !lib(a).settings_return_editor;
            lib(a).reopen_settings = from_settings;
            if !lib(a).settings_return_editor {lib(a).modal=Modal::None;}
            let tool = a.settings.current_tool;
            pensettings_open(a, tool);
        }
        H_SET_CALIB => {
            let from_settings = lib(a).modal == Modal::Settings && !lib(a).settings_return_editor;
            lib(a).reopen_settings = from_settings;
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
            crate::userfonts::userfonts_apply(a); // (Classic's own font, if it has one; Gothic holds none)
            httpd_restyle(ui_theme() == UiTheme::Gothic);
            settings_save(&a.settings);
            redraw(a, RefreshMode::Full);
        }
        H_SET_DARK => {
            a.settings.invert = !a.settings.invert;
            display_set_invert(a.settings.invert);
            live_set_invert(a.settings.invert);
            settings_save(&a.settings);
            redraw(a, RefreshMode::Full);
        }
        H_SET_DISPLAY => {
            let mut dm = std::mem::take(&mut lib(a).dm);
            dispmenu_open(&mut dm, a);
            lib(a).dm = dm;
            open_modal(a, Modal::Display);
        }
        // Settings' Sleep & Clock (2.1): the sleep screen and timer, and the clock, on pages of their own.
        H_SET_SLEEP => {
            let mut dm = std::mem::take(&mut lib(a).dm);
            dispmenu_open_sleep(&mut dm, a);
            lib(a).dm = dm;
            open_modal(a, Modal::Display);
        }
        H_SET_CLOCK => {
            let mut dm = std::mem::take(&mut lib(a).dm);
            dispmenu_open_clock(&mut dm, a);
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
        // What the pen's button does: its own list, as Language's, rather than a row that cycles.
        H_SET_SIDE => open_modal(a, Modal::SideButton),
        H_SIDE_PICK => {
            let pick = clampi(h.arg, 0, 3);
            if a.settings.side_button != pick {
                a.settings.side_button = pick;
                settings_save(&a.settings);
            }
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        H_SIDE_BACK => {
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        // What the eraser does: whole strokes, or precise (settings.ini eraser_mode).
        H_SET_ERASER => open_modal(a, Modal::EraserMode),
        H_ERASER_PICK => {
            let pick = clampi(h.arg, 0, 1);
            if a.settings.eraser_mode != pick {
                a.settings.eraser_mode = pick;
                settings_save(&a.settings);
            }
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        H_ERASER_BACK => {
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        // About: what this Kindle is, the storage used, and its links (their QR pages back to About).
        H_SET_DEVICE => {
            lib(a).device_links_page = 0;
            open_modal(a, Modal::DeviceInfo);
        },
        H_DEVICE_BACK => {
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        H_LINK => {
            lib(a).device_link = clampi(h.arg, 0, ABOUT_LINKS.len() as i32 - 1);
            open_modal(a, Modal::LinkQr);
        }
        H_LINK_PAGE => {
            lib(a).device_links_page += h.arg;
            redraw(a, RefreshMode::Ui);
        }
        H_LINK_BACK => {
            lib(a).modal = Modal::DeviceInfo;
            redraw(a, RefreshMode::Ui);
        }
        H_SET_FONT => {
            // The sheet opens on the font in use: one of the owner's own, or a family. It looks into
            // grimoire/fonts as it opens (new files go to the background check).
            let own = a.settings.font_file.as_str().to_string();
            let own_on = !own.is_empty() && crate::userfonts::userfonts_in_use(&own);
            font_list_scan(a);
            let s = lib(a);
            s.font_picked = false;
            s.font_pick = font_family();
            s.font_file = if own_on { own } else { String::new() };
            s.font_first = match s.fonts.iter().position(|f| f.rel == s.font_file) {
                Some(i) if !s.font_file.is_empty() => i,
                _ => 0,
            };
            s.font_per_page = 1;
            lib(a).modal = Modal::Font;
            font_notice(a);
            redraw(a, RefreshMode::Ui);
        }
        // A font tapped applies at once, saved: a family (and with it a font=file: that does not draw goes),
        // or one of the owner's own fonts.
        H_FONT_PICK => {
            let pick = (h.arg.max(0) as usize).min(FONT_FAMILIES.len() - 1);
            lib(a).font_picked = true; // (a family tapped, even the one that shows: the apply takes font=file: away)
            // (also the family that shows while settings.ini names a font of the owner's own that does not draw
            // now, one a crash kept out or whose file is gone: the tap takes that line away now, not the X later)
            if pick != lib(a).font_pick || !lib(a).font_file.is_empty() || !a.settings.font_file.is_empty() {
                lib(a).font_pick = pick;
                lib(a).font_file.clear();
                // (a font held beside the one in use for the sample goes)
                crate::userfonts::userfonts_sample(a, None);
                if font_apply_pick(a) {
                    redraw(a, RefreshMode::Full);
                }
            }
        }
        H_FONT_OWN => {
            let rel = {
                let s = lib(a);
                s.fonts.get(h.arg.max(0) as usize).filter(|f| f.usable()).map(|f| f.rel.clone())
            };
            if let Some(rel) = rel.filter(|r| *r != lib(a).font_file) {
                lib(a).font_file = rel;
                if font_apply_pick(a) {
                    redraw(a, RefreshMode::Full);
                }
            }
        }
        H_FONT_PAGE => {
            let s = lib(a);
            let per = s.font_per_page.max(1);
            s.font_first = if h.arg < 0 { s.font_first.saturating_sub(per) } else { s.font_first + per };
            redraw(a, RefreshMode::Ui);
        }
        // A tap on a font applies it at once, so X only closes the sheet (a last apply changes nothing then);
        // Back and Esc close it too, the font already the one picked.
        H_FONT_DONE => {
            // (the font applied as it was tapped; the X closes all of Settings too)
            let changed = font_apply_pick(a);
            font_sheet_closed(a);
            close_settings(a, if changed { RefreshMode::Full } else { RefreshMode::Ui });
        }
        H_FONT_CANCEL => {
            font_sheet_closed(a);
            lib(a).modal = Modal::Settings;
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
        H_MORE_MOVE | H_FMORE_MOVE => {
            lib(a).folder_move_page = 0;
            open_modal(a, Modal::Move);
        }
        H_MOVE_PREV | H_MOVE_NEXT => {
            let s = lib(a);
            if h.id == H_MOVE_PREV {
                s.folder_move_page = s.folder_move_page.saturating_sub(1);
            } else if let Some(it) = target_item(s) {
                if (s.folder_move_page + 1) * 6 < move_dests(s, &it).len() {
                    s.folder_move_page += 1;
                }
            }
            redraw(a, RefreshMode::Ui);
        }
        H_MORE_PDF => {
            if let Some(it) = it {
                let id = lib(a).books[it.nb].id.as_str().to_string();
                bg_mirror_pdf(&id, a.dpi, &app_mirror_dir(), true);
                lib(a).modal = Modal::None;
                redraw(a, RefreshMode::Ui);
                app_toast_dismiss(a); // (no toast while it is made: the owner, 2026-11-04; "PDF saved" says when)
            }
        }
        H_MORE_DELETE => {
            open_modal(a, Modal::Delete);
            app_ask(a); // (drawn, not shown yet: its Delete waits until it has been, library_event_in)
        }
        H_MORE_GRAPH | H_FMORE_GRAPH => {
            // (arg: 1 leaves it out, 0 shows it again)
            let omit = h.arg != 0;
            let rc = match it {
                Some(it) if it.is_folder && h.id == H_FMORE_GRAPH => store_set_folder_graph_omit(it.folder.as_str(), omit),
                Some(it) if !it.is_folder && h.id == H_MORE_GRAPH => store_set_graph_omit(lib(a).books[it.nb].id.as_str(), omit),
                _ => -1,
            };
            lib(a).modal = Modal::None;
            reload(a);
            redraw(a, RefreshMode::Ui);
            // (done, the menu row reads the other way next time; no toast: the owner, 2026-11-04, and one left up
            // from before goes)
            if rc != 0 {
                // TRANSLATORS: a toast (about 1700 px at 44 px in bold) after a notebook's or a folder's menu row that
                // leaves its pages out of the Graph of all notebooks, or shows them there again: the change could
                // not be saved.
                app_toast(a, tr("Couldn't save change."), 2500);
            } else {
                app_toast_dismiss(a);
            }
        }
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
        // Set as home, or on the folder that is home, Reset home to Notebooks (2.2, issue #160): saved, and said by
        // the row itself the next time the menu opens (no toast)
        H_FMORE_HOME => {
            if let Some(it) = it.filter(|it| it.is_folder) {
                let home = if h.arg != 0 { Home::Notebooks } else { Home::Library(it.folder.as_str().to_string()) };
                // (a folder's path always reads back as the home it names: store_folder_valid made it)
                if Home::parse(home.value().as_bytes()) == Some(home.clone()) {
                    home_save(a, home);
                }
            }
            lib(a).modal = Modal::None;
            redraw(a, RefreshMode::Ui);
            app_toast_dismiss(a);
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
        H_MERGE_INTO | H_MERGE_BOTH => merge_answer(a, h.id == H_MERGE_BOTH),
        H_MOVE_TOP | H_MOVE_TO if it.is_some_and(|it| it.is_folder) => {
            let it = it.unwrap();
            let parent = if h.id == H_MOVE_TOP { String::new() } else { lib(a).folders[h.arg as usize].clone() };
            if move_folder(a, &it, &parent) {
                lib(a).modal = Modal::Merge;
            } else {
                lib(a).modal = Modal::None;
                reload(a);
            }
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
                    app_toast(a, tr("Couldn't move notebook."), 3000);
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

// ---- the Graph as a place of its own (2.0.0-beta.2, docs/beta2/GRAPH-ALL.md section 10) ----
//
// The owner (2026-11-03): "the graph view is now its own entity and way of managing pages (interlinked ones)
// It needs to be accessible from the main menu through the swipe down with the pages view taken out if
// summoned from the main menu". The Library hosts its own MapView, opened with no notebook (an empty
// NotebookInfo and Nav: mapview_no_notebook): no List or Pages tab. A node's Open goes
// to that notebook at that page in the editor (editor_open_from_graph), and Back from there comes back here
// (map_keep, map_return); Back here returns to the Library.

/// The Library's Graph itself is up (not the notebooks' browser past its caps): its keys are the Graph's.
pub fn library_graph_up(a: &App) -> bool {
    a.scr == ScreenId::Library && a.lib.as_deref().is_some_and(|s| s.map.open && !s.map_picker.open)
}
// The Graph is up, or the notebooks' browser alone (a library past the Graph of all notebooks' caps:
// graph_show).
fn graph_shown(a: &App) -> bool {
    a.scr == ScreenId::Library && a.lib.as_deref().is_some_and(|s| s.map.open || s.map_picker.open)
}

fn with_graph<R>(a: &mut App, f: impl FnOnce(&mut MapView, &mut App) -> R) -> R {
    let mut m = std::mem::take(&mut lib(a).map);
    let r = f(&mut m, a);
    lib(a).map = m;
    r
}

// The Graph opened (every notebook, or keep's pick as keep left it), drawn whole, the Library's toast gone.
// False when it could not be built: it has said why, and the Library stays.
fn graph_open(a: &mut App, keep: Option<MapKeep>) -> bool {
    app_toast(a, "", 0);
    graph_show(a, keep)
}

// graph_open keeping the toast up (a node's notebook that did not open has said so).
fn graph_show(a: &mut App, keep: Option<MapKeep>) -> bool {
    let s = lib(a);
    s.modal = Modal::None;
    s.pressed = None;
    s.map_picker.open = false;
    let (nb, nav) = (NotebookInfo::default(), Nav::default());
    let pick = keep.and_then(|k| k.pick);
    let ok = with_graph(a, |m, a| mapview_open_scoped(m, a, &nb, &nav, 0, false, pick.as_ref().map(|p| p.as_str())));
    if !ok {
        let too_big = lib(a).map.too_big;
        mapview_close(&mut lib(a).map);
        // Too many linked pages for every notebook (it has said so): the notebooks' browser opens, to show one
        // (the only notebook choice left: the owner removed the Notebook chip, 2026-11-03); Back from it is the
        // Library.
        if too_big && pick.is_none() && lib(a).map_picker.start_notebook("") {
            a.toast_floor = 0;
            redraw(a, RefreshMode::Full);
            input_touch_context(library_touch_view(a));
            return true;
        }
        redraw(a, RefreshMode::Full);
        return false;
    }
    if let Some(k) = keep {
        with_graph(a, |m, a| mapview_restore(m, a, &nav, &k));
    }
    a.toast_floor = 0;
    input_touch_context(library_touch_view(a));
    true
}

// Back (or two fingers down again): the Library, as it was.
fn graph_close(a: &mut App) {
    let s = lib(a);
    mapview_close(&mut s.map);
    s.map_picker.open = false;
    s.map_keep = None;
    redraw(a, RefreshMode::Full);
}

fn graph_draw(a: &mut App) {
    if lib(a).map_picker.open {
        let mut p = std::mem::take(&mut lib(a).map_picker);
        p.draw(a);
        lib(a).map_picker = p;
        return;
    }
    let full = Rect::new(0, 0, a.w, a.h);
    with_graph(a, |m, a| mapview_draw(m, a, &Nav::default(), full));
}

// A notebook picked in the browser past the caps (graph_show): the Graph opens on it, or the Library stays.
fn graph_pick(a: &mut App, pick: &str) {
    let mut keep = mapview_keep(&lib(a).map);
    keep.pick = Some(FixedStr::from_str(pick));
    if !graph_open(a, Some(keep)) {
        graph_close(a);
    }
}

fn graph_result(a: &mut App, r: MapResult) {
    match r {
        MapResult::Close => graph_close(a),
        MapResult::Target(t) => {
            // A node's Open: that notebook at that page, the Graph kept for Back.
            let Some((id, pid)) = nav_external_target(t.as_str()) else { return };
            let (id, keep) = (id.to_string(), mapview_keep(&lib(a).map));
            mapview_close(&mut lib(a).map);
            lib(a).map_keep = Some(keep);
            crate::editor::editor_open_from_graph(a, &id, pid);
            if a.scr == ScreenId::Library {
                // (it did not open, and has said so: the Graph again, its toast kept)
                lib(a).map_keep = None;
                graph_show(a, Some(keep));
            }
        }
        // (no notebook is open: nothing here names, renames or repairs its pages, and there is no List or
        // Pages tab to ask for them)
        MapResult::Open(_) | MapResult::Repair(..) | MapResult::Rename(_) | MapResult::Pages | MapResult::List
        | MapResult::None => {}
    }
}

fn graph_event(a: &mut App, ev: &InEvent) {
    let ph = app_pointer_phase(ev);
    if lib(a).map_picker.open {
        if ph < 0 {
            return;
        }
        let mut p = std::mem::take(&mut lib(a).map_picker);
        let r = p.pointer(a, ph, ev);
        lib(a).map_picker = p;
        match r {
            NotebookPick::Notebook(id) => graph_pick(a, &id),
            NotebookPick::Cancel => redraw(a, RefreshMode::Full),
            _ => {}
        }
        return;
    }
    if ev.type_ == InType::Gesture {
        // The Graph's own gesture (two fingers down, or the way back up) closes it, as over a notebook.
        let Some(key) = gesturemap_key_of_id(ev.value) else { return };
        let m = &a.gestures;
        if gesturemap_action(m, key) == SHORTCUT_INDEX_MAP || gesturemap_action(m, gesturemap_mirror(key)) == SHORTCUT_INDEX_MAP {
            graph_close(a);
        }
        return;
    }
    if matches!(ev.type_, InType::PinchBegin | InType::PinchMove | InType::PinchEnd) {
        with_graph(a, |m, a| mapview_pinch(m, a, ev));
        return;
    }
    if ph < 0 {
        return;
    }
    let r = with_graph(a, |m, a| mapview_pointer(m, a, &Nav::default(), ph, ev));
    graph_result(a, r);
}

// ---- the file picker (2.1.0-beta.2, browser.rs) ----
//
// New notebook > From a file... opens Files in pick mode over New notebook's sheet, which stays under it. The
// picker draws and takes the contacts, and says what was asked (BrowserResult); this does it: a PDF or a folder
// of pictures is queued as an import (named as typed in the sheet, else by its file; into the sheet's In: folder),
// a PDF a notebook already holds opens that notebook, Back at the picker's root comes back to the sheet.

/// The file picker is up: its keys and contacts are its own.
fn files_shown(a: &App) -> bool {
    a.scr == ScreenId::Library && a.lib.as_deref().is_some_and(|s| s.files.open)
}

/// Files is up and its keys, contacts and ticks are its own: always, but while "Exit Grimoire?" is asked over it
/// (at a home inside Files, 2.2, issue #160: `files_back`), when the question takes them as it does over the
/// Library's top. (A picker is over New notebook's sheet or the Templates sheet, never over that question.)
fn files_in_front(a: &App) -> bool {
    files_shown(a) && a.lib.as_deref().is_some_and(|s| s.modal != Modal::Exit)
}

/// Whether the Import place is on screen: the staying notice of an import stays off it (its rows say the same).
/// Whether the Library is the screen, its grid of the folder `folder` showing with nothing over it (no sheet, no
/// picker, no Graph): an import's new card is then seen as it arrives, and no toast says it (D20).
pub fn library_shows_folder(a: &App, folder: &str) -> bool {
    a.scr == ScreenId::Library
        && !a.asleep
        && a.lib.as_deref().is_some_and(|s| s.modal == Modal::None && !s.files.open && !s.map.open && !s.map_picker.open && s.cur_folder.as_str() == folder)
}

/// The folder of the Templates sheet in front over New notebook (the picker not over it), None for none: a copy into
/// it that ends is shown there, with no toast (app.rs).
pub fn library_sheet_folder(a: &App) -> Option<crate::bgwork_copy::CopyTo> {
    let s = a.lib.as_deref().filter(|s| a.scr == ScreenId::Library && s.modal == Modal::Templates && s.ts.open && !s.files.open)?;
    Some(tplsheet_folder(&s.ts))
}

pub fn library_import_place_up(a: &App) -> bool {
    files_shown(a) && a.lib.as_deref().is_some_and(|s| s.files.import_place_up())
}

fn with_files<R>(a: &mut App, f: impl FnOnce(&mut Browser, &mut App) -> R) -> R {
    let mut b = std::mem::take(&mut lib(a).files);
    let r = f(&mut b, a);
    lib(a).files = b;
    r
}

/// What the browser is handed of the settings for Files.
fn files_prefs(a: &App) -> FilesPrefs {
    FilesPrefs { card: a.settings.files_card, sort: a.settings.files_sort, show: a.settings.files_show }
}

fn files_open(a: &mut App) {
    let prefs = files_prefs(a);
    let ok = with_files(a, |b, _| b.open(PickSpec::import(), prefs));
    files_opened(a, ok);
}

/// The Templates sheet's From a file... (2.2, #146): the picture pick over the sheet, which stays under it (Back at
/// the picker's root, and a copy asked, come back to it).
fn files_open_picture(a: &mut App, to: crate::bgwork_copy::CopyTo) {
    let prefs = files_prefs(a);
    let ok = with_files(a, |b, _| b.open(PickSpec::picture(to), prefs));
    files_opened(a, ok);
}

/// Files, the place (2.2): from the Files card, its menu's Open, and the `files` action (whether or not the card is
/// in the Library), over the Library as it is.
fn files_open_place(a: &mut App) {
    let ok = files_place(a);
    files_opened(a, ok);
}

/// Files, the place, opened at its root with what the settings say of it, and nothing drawn: false when there is no
/// storage to browse.
fn files_place(a: &mut App) -> bool {
    let prefs = files_prefs(a);
    let view = a.settings.files_view;
    let home = home_of(a);
    with_files(a, |b, _| {
        let ok = b.open(PickSpec::place(), prefs);
        // (Files' View, the list or Covers: the place's alone, a picker is always a list)
        b.set_view(view);
        // (home as settings.ini has it, 2.2, issue #160)
        b.set_home(home);
        ok
    })
}

fn files_opened(a: &mut App, ok: bool) {
    if !ok {
        // (no storage to browse: the sheet stays)
        app_toast(a, tr("Couldn't open folder."), 3000);
        return;
    }
    lib(a).pressed = None;
    a.toast_floor = 0;
    redraw(a, RefreshMode::Full);
    input_touch_context(library_touch_view(a));
}

/// The picker closed: back to New notebook's sheet (a cancel) or the Library (an import started). Files (place
/// mode) always goes back to the Library.
fn files_close(a: &mut App, to_library: bool) {
    let s = lib(a);
    let to_library = to_library || s.files.mode == BrowseMode::Place;
    s.files.close();
    if to_library {
        s.modal = Modal::None;
    }
    app_notice_clear(a, NoticeKey::Loading);
    app_notice_clear(a, NoticeKey::Previews);
    if to_library {
        reload(a);
    }
    redraw(a, RefreshMode::Full);
}

/// What a toast says when an import that was asked for has to wait for another: a file picked while an import of
/// an earlier run is still going, and a PDF KOReader handed over by its path behind any other import (app.rs).
pub fn library_import_waits_text() -> &'static str {
    // TRANSLATORS: a toast after a file was picked to import while an import from an earlier run is
    // still going (in the background): this one starts once that one is done. One line of about
    // 1700 px at 44 px, or two.
    tr("Another import is still going. This one starts after it.")
}

fn files_result(a: &mut App, r: BrowserResult) {
    use crate::pdfimport::Queued;
    let (name, folder) = {
        let s = lib(a);
        (
            // (Files has no name typed: an import from it is named by its file)
            (s.files.mode == BrowseMode::Pick && s.new_name_typed && !s.new_name.is_empty()).then(|| s.new_name.as_str().to_string()),
            (!s.cur_folder.is_empty()).then(|| s.cur_folder.as_str().to_string()),
        )
    };
    match r {
        BrowserResult::None => {}
        BrowserResult::Cancel => files_close(a, false),
        BrowserResult::Pdf(ref p) | BrowserResult::Pictures(ref p) => {
            let pdf = matches!(r, BrowserResult::Pdf(_));
            let q = if pdf {
                crate::pdfimport::pdf_import_queue(p, name.as_deref(), folder.as_deref(), a.dpi)
            } else {
                crate::bgwork_template::import_pick_queue(p, name.as_deref(), folder.as_deref(), a.dpi)
            };
            files_close(a, true);
            // (a start shows on the staying notice; a wait is said, as nothing else would show it for a while: it
            // is queued once the other import has ended)
            match q {
                Queued::Refused(_) => {
                    let mut msg = String::new();
                    if pdf {
                        csnprintf!(&mut msg, 256, tr("Couldn't import PDF (see %s)."), "grimoire.log");
                    } else {
                        csnprintf!(&mut msg, 256, tr("Couldn't import page folder (see %s)."), "grimoire.log");
                    }
                    app_toast(a, &msg, 4000);
                }
                Queued::Waiting => app_toast(a, library_import_waits_text(), 6000),
                _ => {}
            }
        }
        BrowserResult::OpenNotebook(id) => {
            files_close(a, true);
            editor_open_notebook(a, &id);
        }
        // Files' Sort, Show or View chip: kept for the next time Files opens
        BrowserResult::Prefs => {
            let (prefs, view) = (lib(a).files.prefs, lib(a).files.view());
            a.settings.files_sort = prefs.sort;
            a.settings.files_show = prefs.show;
            a.settings.files_view = view;
            settings_save(&a.settings);
        }
        // "Show Files in Library": the card is the Library's last from now on (the browser drew its root without the row)
        BrowserResult::ShowInLibrary => {
            a.settings.files_card = true;
            settings_save(&a.settings);
            reload(a);
        }
        // A picture's Use as template or Use as photo (#146): the copy job runs. Files stays as it is (the app's tick
        // says when the copy ends); the picture pick goes back to the Templates sheet under it, whose From a file...
        // tile waits for the copy and then shows the new template
        BrowserResult::Copying(_) => {
            if lib(a).files.mode == BrowseMode::Pick {
                tplsheet_copy_asked(&mut lib(a).ts);
                files_close(a, false);
            }
        }
        // Files' Restore put a notebook back (2.2, issue #144): the Library under Files is read again at once
        BrowserResult::NotebookBack(_) => reload(a),
        // Home (2.2, issue #160). The Go to sheet's Home row: home, wherever it is
        BrowserResult::GoHome => library_home(a),
        // Set as home, on the folder shown or a folder's card: the folder is looked at again through the walk, as
        // a home read from settings.ini is, before it is kept; one that is no longer a folder to open at is said
        // as a folder that does not open. Nothing else is said: the folder that is home has no Back, and the rows
        // read the other way
        BrowserResult::SetHome(p) => {
            if home_folder_there(&p) {
                home_save(a, Home::Files(p));
                app_toast_dismiss(a);
            } else {
                app_toast(a, tr("Couldn't open folder."), 3000);
            }
            redraw(a, RefreshMode::Ui);
        }
        // Reset home to Notebooks: the Library's top again from the next start; Files stays where it is
        BrowserResult::ResetHome => {
            home_save(a, Home::Notebooks);
            redraw(a, RefreshMode::Ui);
            app_toast_dismiss(a);
        }
        BrowserResult::LookAgain => {
            import_scan(a.dpi);
            with_files(a, |b, a| {
                b.relist(now_ms());
                b.draw(a);
            });
            app_invalidate_all(a, RefreshMode::Ui);
        }
    }
}

fn files_event(a: &mut App, ev: &InEvent) {
    if ev.type_ == InType::Gesture {
        // (a gesture bound to files or home acts as its key does here; two fingers down does nothing: the Graph's
        // gesture belongs to the Notebooks place)
        let action = gesturemap_key_of_id(ev.value).map_or(0, |k| gesturemap_action(&a.gestures, k));
        if action == SHORTCUT_FILES || action == SHORTCUT_HOME {
            library_shortcut(a, action);
        }
        return;
    }
    let ph = app_pointer_phase(ev);
    if ph < 0 {
        return;
    }
    let r = with_files(a, |b, a| b.pointer(a, ph, ev));
    files_result(a, r);
}

/// Esc: the card closes, else up one level, and at the root the picker cancels. At the folder that is home (2.2,
/// issue #160), with nothing over its rows, nothing is further up: it asks "Exit Grimoire?", as at the Library's
/// top; a home folder that no longer opens (gone over USB while it was shown) is left upward as any folder that is
/// gone (`Browser::at_home_root`). Always taken.
fn files_back(a: &mut App) -> bool {
    if lib(a).files.at_home_root() {
        lib(a).exit_no = false;
        open_modal(a, Modal::Exit);
        return true;
    }
    if !with_files(a, |b, a| b.back(a)) {
        files_close(a, false);
    }
    true
}

fn files_tick(a: &mut App, now: u32) {
    with_files(a, |b, a| b.tick(a, now));
}

// Two fingers down over the Library (or whatever gestures.ini binds to index-map) open the Graph; a gesture bound to
// files or home (2.2) does that; no other gesture does anything here.
fn library_gesture(a: &mut App, ev: &InEvent) {
    if lib(a).modal != Modal::None {
        return;
    }
    let Some(key) = gesturemap_key_of_id(ev.value) else { return };
    match gesturemap_action(&a.gestures, key) {
        SHORTCUT_INDEX_MAP => {
            graph_open(a, None);
        }
        // (Files' two, 2.2, when gestures.ini binds them)
        SHORTCUT_FILES => files_open_place(a),
        SHORTCUT_HOME => library_home(a),
        _ => {}
    }
}

/// A key's or a controller's action on the Library (app.rs, before the global actions): index-map opens the
/// Graph, or closes it; over the Graph, go-back and close-menu go back (from its notebooks' browser past the caps
/// to the Library, from the Graph to the Library). True when it was taken.
pub fn library_shortcut(a: &mut App, action: i32) -> bool {
    if a.scr != ScreenId::Library || a.lib.is_none() {
        return false;
    }
    lib(a).home_pending = false; // (a key's or a button's action on the main menu: it is in use)
    if action == SHORTCUT_SAVE {
        library_save(a);
        return true;
    }
    if files_in_front(a) {
        match action {
            SHORTCUT_BACK | SHORTCUT_GO_BACK | SHORTCUT_CLOSE_MENU => return files_back(a),
            SHORTCUT_PREV_PAGE | SHORTCUT_NEXT_PAGE => {
                with_files(a, |b, a| b.turn_key(a, action == SHORTCUT_NEXT_PAGE));
                return true;
            }
            // (2.2: files goes to Files' root, home to the home folder: out of Files unless it is inside it)
            SHORTCUT_FILES => {
                with_files(a, |b, a| b.to_root(a));
                return true;
            }
            SHORTCUT_HOME => {
                library_home(a);
                return true;
            }
            _ => return false,
        }
    }
    if graph_shown(a) {
        // The Graph's own keys first: pans, zoom, Fit, its view, Select all, Enter, and Esc's selection.
        if lib(a).map.open && !lib(a).map_picker.open {
            if let Some(r) = with_graph(a, |m, a| mapview_action(m, a, &Nav::default(), action)) {
                graph_result(a, r);
                return true;
            }
        }
        if !matches!(action, SHORTCUT_INDEX_MAP | SHORTCUT_GO_BACK | SHORTCUT_CLOSE_MENU | SHORTCUT_BACK) {
            return false;
        }
        if lib(a).map_picker.open && action != SHORTCUT_INDEX_MAP {
            lib(a).map_picker.open = false;
            redraw(a, RefreshMode::Full);
        } else {
            graph_close(a);
        }
        return true;
    }
    if action == SHORTCUT_BACK {
        return library_back(a);
    }
    // What's new's list and a version's notes: Left and Right turn their pages, as their arrows.
    if lib(a).modal == Modal::About && matches!(action, SHORTCUT_PREV_PAGE | SHORTCUT_NEXT_PAGE) {
        let id = if action == SHORTCUT_NEXT_PAGE { H_ABOUT_NEXT } else { H_ABOUT_PREV };
        act(a, &Hit { id, ..Default::default() });
        return true;
    }
    if lib(a).modal != Modal::None {
        return false;
    }
    match action {
        SHORTCUT_INDEX_MAP => {
            graph_open(a, None);
        }
        // The top bar's buttons, from the keyboard: Ctrl+N, Ctrl+F, S and O.
        SHORTCUT_NEW_NOTEBOOK => act(a, &Hit { id: H_NEW, ..Default::default() }),
        SHORTCUT_NEW_FOLDER => act(a, &Hit { id: H_NEWFOLDER, ..Default::default() }),
        SHORTCUT_SETTINGS => act(a, &Hit { id: H_SETTINGS, ..Default::default() }),
        SHORTCUT_SHARE => act(a, &Hit { id: H_SHARE, ..Default::default() }),
        // Files' two (2.2, no default key): Files, whether or not its card is in the Library; home, the home folder
        SHORTCUT_FILES => files_open_place(a),
        SHORTCUT_HOME => library_home(a),
        // Left and Right: the list's previous and next screen, as its arrows (while there is one).
        SHORTCUT_PREV_PAGE | SHORTCUT_NEXT_PAGE => {
            let want = if action == SHORTCUT_NEXT_PAGE { H_NEXT } else { H_PREV };
            if lib(a).hits.h.iter().any(|h| h.id == want) {
                turn(a, want == H_NEXT);
            }
        }
        _ => return false,
    }
    true
}

/// The `home` action (2.2), and the Go to sheet's Home row: the home folder (issue #160), which is the Library's
/// top at its first screen until one is set. Out of Files first, unless home is inside it and Files is up as the
/// place: it goes there itself.
fn library_home(a: &mut App) {
    let stays = matches!(home_of(a), Home::Files(_)) && !a.settings.home_refused && lib(a).files.mode == BrowseMode::Place;
    if files_shown(a) && !stays {
        files_close(a, true);
    }
    home_go(a);
    redraw(a, RefreshMode::Full);
    input_touch_context(library_touch_view(a));
}

/// Home as settings.ini has it (2.2, issue #160; home.rs): Notebooks, the Library's top, without the line.
fn home_of(a: &App) -> Home {
    Home::parse(a.settings.home.as_bytes()).unwrap_or_default()
}

/// Home is `home` from now on: settings.ini has it (no line for Notebooks), and Files is told.
fn home_save(a: &mut App, home: Home) {
    a.settings.home.set(&home.value());
    a.settings.home_refused = false;
    settings_save(&a.settings);
    lib(a).files.set_home(home);
}

/// Whether the Library, as it comes in front now, opens at home (2.2, issue #160): the main menu has not been used
/// yet in this run, and a home is set (or one was, that settings.ini's line no longer names). With no home set
/// nothing here does anything: such a start is as it always was.
fn home_due(a: &App, relayout: bool) -> bool {
    !relayout && a.lib.as_deref().is_some_and(|s| s.home_pending) && (a.settings.home_refused || !a.settings.home.is_empty())
}

/// Whether a home inside Files waits as the Library comes in front at a start (the Library's cards show
/// meanwhile). For What's new, open over the Library at the first start after an update: its closing goes home
/// (close_modal). And once for a notebook KOReader started Grimoire on (a page to annotate, a book): it opens over
/// the Library's cards as before (app.rs, book_tick, which opens nothing over Files), and home opens when the main
/// menu next comes in front; a hand-over that opens no notebook opens home at once (library_handover_tried). (The
/// hand-over's one wait is taken at the first start even under What's new, so that the main menu in front after
/// that notebook opens at home.)
fn home_waits(a: &mut App) -> bool {
    if a.settings.home_refused || !matches!(home_of(a), Home::Files(_)) {
        return false;
    }
    let s = lib(a);
    let held = app_handed_over() && !std::mem::replace(&mut s.home_held, true);
    if s.modal == Modal::About {
        s.home_deferred = true;
        return true;
    }
    held
}

/// KOReader's hand-over at a start has been tried (app_init, after annotate_at_start and book_start): when it opened
/// no notebook and none is on its way (`opening` false: the page did not open, no book is being put in), a home
/// inside Files that waited for that notebook (home_waits) opens now, as at a plain start. (2.2, issue #160)
pub fn library_handover_tried(a: &mut App, opening: bool) {
    let waited = a.lib.as_deref().is_some_and(|s| s.home_held && s.home_pending && s.modal == Modal::None && !s.files.open);
    if opening || a.scr != ScreenId::Library || !waited {
        return;
    }
    home_go(a);
    redraw(a, RefreshMode::Full);
    input_touch_context(library_touch_view(a));
}

/// Goes home (2.2, issue #160) and reads the Library's lists; the caller draws. The Library's top, one of its
/// folders, or a folder inside Files, which opens there over the Library as it was. A home that is not there, or
/// that Grimoire does not enter, is `home_lost`. A Library folder is a name in the Library's own list, just read;
/// a folder inside Files is checked through fsview's walk before Files is opened at it (`home_files`).
fn home_go(a: &mut App) {
    let refused = std::mem::take(&mut a.settings.home_refused);
    let home = (!refused).then(|| home_of(a));
    let s = lib(a);
    match &home {
        Some(Home::Files(_)) => {}
        Some(Home::Library(f)) => {
            s.cur_folder.set(f);
            s.page = 0;
        }
        _ => {
            s.cur_folder.clear();
            s.page = 0;
        }
    }
    reload(a);
    let found = match &home {
        None => false,
        Some(Home::Notebooks) => true,
        Some(Home::Library(f)) => lib(a).folders.iter().any(|x| x == f),
        Some(Home::Files(p)) => home_files(a, p),
    };
    if !found {
        home_lost(a);
    }
}

/// Files, the place, at the folder `p` that is home: opened there, or taken there when it is up. False, with
/// nothing opened at `p`, when the walk does not reach it as a folder that can be home (`home_folder_there`,
/// looked at before Files is opened).
fn home_files(a: &mut App, p: &StoragePath) -> bool {
    if !home_folder_there(p) {
        return false;
    }
    let up = lib(a).files.open && lib(a).files.mode == BrowseMode::Place;
    if !(up || files_place(a)) {
        return false;
    }
    if !with_files(a, |b, _| b.enter_home(p)) {
        return false;
    }
    lib(a).pressed = None;
    a.toast_floor = 0;
    true
}

/// Home was gone or refused when Grimoire went there (2.2, issue #160): the Library's top is the place shown,
/// "Home folder not found." is said once, and home is Notebooks again in settings.ini, at once. The value that
/// failed is not kept: kept, it would be tried and said at every start, with no row in reach to reset it for
/// someone who never shows Files. (The screen does not say why it changed.)
fn home_lost(a: &mut App) {
    if lib(a).files.open && lib(a).files.mode == BrowseMode::Place {
        lib(a).files.close();
        app_notice_clear(a, NoticeKey::Loading);
        app_notice_clear(a, NoticeKey::Previews);
    }
    let s = lib(a);
    let moved = !s.cur_folder.is_empty();
    s.cur_folder.clear();
    s.page = 0;
    home_save(a, Home::Notebooks);
    if moved {
        reload(a);
    }
    qlog!("home: the home folder was not found: home is Notebooks again");
    // TRANSLATORS: a toast as Grimoire starts (or goes home): the folder set as home is not there any more, or is
    // not one Grimoire opens, so the Library's top level shows and home is Notebooks again. One line of about
    // 1000 px at 44 px in bold.
    app_toast(a, tr("Home folder not found."), 4000);
}

/// Whether the Graph (or its notebooks' browser) is up over the Library: a key pressed now is the Graph's.
pub fn library_graph_shown(a: &App) -> bool {
    graph_shown(a)
}

/// Ctrl+S in the Library: anything waiting is written (the settings), with no toast (one left up from before goes).
fn library_save(a: &mut App) {
    settings_save(&a.settings);
    app_toast_dismiss(a);
}

/// Esc (`back`) on the Library, or on its Settings over a notebook: the first
/// layer in front eats it. A name being typed is cancelled; a dialog or card is answered No (Cancel); a sheet
/// goes back one level (each Settings sheet back to Settings, then Settings closed; New notebook's Templates
/// and Custom music back to New notebook); with nothing open, a folder goes up to the top level, and the top
/// level asks "Exit Grimoire?". False when there was nothing to do.
pub fn library_back(a: &mut App) -> bool {
    if a.lib.is_none() {
        return false;
    }
    if lib(a).share_wifi_return && lib(a).modal == Modal::Wifi {
        lib(a).share_wifi_return = false;
        app_switch(a, ScreenId::Share);
        return true;
    }
    let tap = |a: &mut App, id: i32| act(a, &Hit { id, ..Default::default() });
    if files_in_front(a) {
        return files_back(a);
    }
    if library_record_active(a) {
        tap(a, H_RECORD_CANCEL);
        return true;
    }
    let (modal, in_folder, reset_asked) = {
        let s = lib(a);
        (s.modal, !s.cur_folder.is_empty(), s.gestures_reset_confirm)
    };
    match modal {
        Modal::Kb => {
            if !app_type_text(a, KbInput::Cancel) {
                keyboard_close(&mut lib(a).kb);
                kb_finished(a, false);
            }
        }
        Modal::None if a.scr != ScreenId::Library => return false,
        Modal::None if in_folder => tap(a, H_BACK),
        Modal::None => {
            lib(a).exit_no = false;
            open_modal(a, Modal::Exit);
        }
        Modal::Templates => {
            tplsheet_close(&mut lib(a).ts);
            lib(a).modal = Modal::New;
            redraw(a, RefreshMode::Ui);
        }
        Modal::Music => {
            musicbuild_free(&mut lib(a).mb);
            lib(a).modal = Modal::New;
            redraw(a, RefreshMode::Ui);
        }
        Modal::Display => {
            lib(a).dm.open = false;
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        Modal::LinkQr => {
            lib(a).modal = Modal::DeviceInfo;
            redraw(a, RefreshMode::Ui);
        }
        Modal::Language | Modal::Koreader | Modal::Shortcuts | Modal::PairMenu | Modal::Updates | Modal::Wifi
        | Modal::SideButton | Modal::EraserMode | Modal::DeviceInfo => {
            lib(a).modal = Modal::Settings;
            redraw(a, RefreshMode::Ui);
        }
        Modal::Pair | Modal::Buttons => tap(a, H_BUTTONS_DONE),
        Modal::Font => tap(a, H_FONT_CANCEL),
        // (What's new, one layer a press: a version's notes back to the list, the list back to the card
        // that opened it, as the Templates sheet goes back to New notebook; the card closes)
        Modal::About if lib(a).about_view == AboutView::Note => tap(a, H_ABOUT_BACK),
        Modal::About if lib(a).about_view == AboutView::List => {
            lib(a).about_view = AboutView::Card;
            about_step(a);
        }
        Modal::Gestures if reset_asked => tap(a, H_GESTURES_RESET_CANCEL),
        Modal::Gestures => tap(a, H_GESTURES_DONE),
        Modal::New | Modal::Settings | Modal::More | Modal::FolderMore | Modal::Move | Modal::Delete
        | Modal::About | Modal::Exit | Modal::Merge | Modal::FilesMore => close_modal(a),
    }
    true
}

/// A keyboard's key while a Library dialog takes keys of its own (section 9), before any shortcut: Exit
/// Grimoire?'s Enter (the highlighted answer), Left and Right (the highlight) and Esc (No), and New notebook's
/// Enter (Create). True when it took the key; Exit Grimoire? takes every key (nothing behind it acts).
pub fn library_modal_key(a: &mut App, code: i32) -> bool {
    if a.scr != ScreenId::Library || graph_shown(a) || files_in_front(a) || library_record_active(a) {
        return false;
    }
    if code == 1 && lib(a).share_wifi_return && lib(a).modal == Modal::Wifi {
        lib(a).share_wifi_return = false;
        app_switch(a, ScreenId::Share);
        return true;
    }
    let Some(s) = a.lib.as_deref_mut() else { return false };
    const ENTER: i32 = 28;
    const KP_ENTER: i32 = 96;
    match s.modal {
        Modal::Exit => {
            match code {
                ENTER | KP_ENTER => {
                    let id = if s.exit_no { H_EXIT_NO } else { H_EXIT_YES };
                    act(a, &Hit { id, ..Default::default() });
                }
                105 | 106 => {
                    s.exit_no = !s.exit_no;
                    redraw(a, RefreshMode::Ui);
                }
                1 => close_modal(a),
                _ => {}
            }
            true
        }
        Modal::New if code == ENTER || code == KP_ENTER => {
            act(a, &Hit { id: H_NEW_CREATE, ..Default::default() });
            true
        }
        // What's new's Enter opens: the card's Read all (the list), and on the list the notes of the row
        // marked as this version's (the card's notes; the newest when no row is this version's).
        Modal::About if (code == ENTER || code == KP_ENTER) && s.about_view != AboutView::Note => {
            let hit = if s.about_view == AboutView::Card {
                Hit { id: H_ABOUT_MORE, ..Default::default() }
            } else {
                let all = crate::about::notes();
                Hit { id: H_ABOUT_ROW, arg: crate::about::current(all, GRIMOIRE_VERSION) as i32, ..Default::default() }
            };
            act(a, &hit);
            true
        }
        _ => false,
    }
}

fn graph_tick(a: &mut App, now: u32) {
    if lib(a).map_picker.open {
        return;
    }
    let (nb, nav) = (NotebookInfo::default(), Nav::default());
    with_graph(a, |m, a| {
        mapview_tick(m, a, now);
        // (another notebook picked: other notebooks' links to it, as the editor's Graph refreshes them)
        mapview_anchors_refresh(m, &nb, &nav);
        // what the library index learned since: new nodes placed beside the kept ones
        mapview_library_refresh(m, a, &nb, &nav, now);
    });
    // The caches the Graph reads, kept on quiet ticks as the editor keeps them (no notebook may have been
    // opened since Grimoire started: the index's first check runs here then).
    if a.input_drained && now.wrapping_sub(a.last_input) > 250 && !backlinks_step() {
        libindex_step();
    }
    if lib(a).map.needs_redraw {
        redraw(a, RefreshMode::Ui);
    }
}

/// A notebook the Graph opened is closing back to it (editor.rs: Back with nothing before it, or the Library
/// button): the Library shows the Graph again as it enters.
pub fn library_return_to_graph(a: &mut App) {
    if let Some(s) = a.lib.as_deref_mut() {
        s.map_return = s.map_keep.is_some();
    }
}

/// A layout child for the Graph opened here ended (app.rs routes it here first): adopted at its next quiet
/// tick. False when this Graph is not open (the editor's then).
pub fn library_map_done(a: &mut App, id: &str, ok: bool) -> bool {
    match a.lib.as_deref_mut().filter(|s| s.map.open) {
        Some(s) => {
            mapview_background_done(&mut s.map, id, ok);
            true
        }
        None => false,
    }
}
/// The same for the Graph of all notebooks' child.
pub fn library_map_all_done(a: &mut App, ok: bool) -> bool {
    match a.lib.as_deref_mut().filter(|s| s.map.open) {
        Some(s) => {
            mapview_all_background_done(&mut s.map, ok);
            true
        }
        None => false,
    }
}
/// A node of the Graph opened here, for the runner's `tap map-node`.
pub fn library_map_point(a: &App, spec: &str) -> Option<(f32, f32)> {
    a.lib.as_deref().filter(|s| s.map.open).and_then(|s| mapview_node_point(&s.map, spec))
}

// ---- the screen interface (app.h, DECLARE_SCREEN) ----

pub fn library_init(a: &mut App) {
    a.lib = Some(Box::default());
    let s = lib(a);
    s.target = -1;
    s.learn_row = -1;
    s.buttons_group = -1;
    s.gestures_group = -1;
    s.home_pending = true;
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
    if !relayout {
        if lib(a).modal == Modal::Font {
            font_sheet_closed(a);
        }
        // The font settings.ini asks for, if its check ended while a notebook was open: from now on (never in
        // the middle of a page).
        if crate::userfonts::userfonts_waiting() && crate::userfonts::userfonts_apply(a) {
            httpd_restyle(ui_theme() == UiTheme::Gothic);
        }
    }
    let s = lib(a);
    let reopen_settings = std::mem::take(&mut s.reopen_settings);
    if !relayout {
        s.modal = Modal::None;
        s.settings_return_editor = false;
        s.share_wifi_return = false;
    }
    if reopen_settings {
        // (Back from Pen feel or Calibration, which Settings opened: Settings again, as it was left.)
        s.modal = Modal::Settings;
    }
    s.pressed = None;
    // What's new opens by itself once after every update (Settings.whatsnew_due, the owner's rule of
    // 2026-11-05): over the Library the first time it shows, this version's notes then recorded as
    // seen. Dismiss, Back and Esc close it; the bat opens it again.
    if !relayout && a.settings.whatsnew_due {
        a.settings.whatsnew_due = false;
        a.settings.whatsnew_seen.set(GRIMOIRE_VERSION);
        settings_save(&a.settings);
        let s = lib(a);
        about_open(s);
        s.modal = Modal::About;
    }
    // Home (2.2, issue #160): while the main menu has not been used in this run, it opens at home as it comes in
    // front. With no home set this is the plain reload it always was.
    if !relayout {
        lib(a).home_deferred = false;
    }
    if home_due(a, relayout) && !home_waits(a) {
        home_go(a);
    } else {
        reload(a);
    }
    // PDFs copied into grimoire/import/ while Grimoire ran: looked at when the Library comes in front, at most
    // every 10 s (pdfimport.rs; not on a re-layout).
    if !relayout {
        crate::app::app_import_rescan(a, true);
    }
    // (Back from a notebook the Graph opened: the Graph again, as it was left)
    if !relayout && std::mem::take(&mut lib(a).map_return) {
        if let Some(keep) = lib(a).map_keep.take() {
            graph_open(a, Some(keep));
        }
    }
    input_touch_context(library_touch_view(a));
    redraw(a, RefreshMode::Full);
}

pub fn library_leave(a: &mut App) {
    if !a.relayout && a.lib.as_deref().is_some_and(|s| s.files.open) {
        lib(a).files.close();
        app_notice_clear(a, NoticeKey::Loading);
    }
    if !a.relayout && a.lib.as_deref().is_some_and(|s| s.map.open || s.map_picker.open) {
        // (the Graph saves its layout as it closes; a notebook it opened has kept what Back needs)
        let s = lib(a);
        mapview_close(&mut s.map);
        s.map_picker.open = false;
    }
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

/// What touchgest needs of the Library (input_touch_context): whether gestures are on, whether one is
/// recorded, whether the keyboard is up (its keys take two fingers rolling over one another), and whether a
/// spread zooms: over the Graph (its notebooks' browser aside), with Pinch zoom on, as over the
/// editor's Graph.
pub fn library_touch_view(a: &App) -> TgView {
    let record = library_record_active(a);
    let kb = a.lib.as_deref().filter(|s| s.modal == Modal::Kb).map(|s| s.kb.area);
    let graph = graph_shown(a) && a.lib.as_deref().is_some_and(|s| !s.map_picker.open);
    TgView {
        gestures_on: record || a.settings.gestures,
        record,
        keyboard_open: kb.is_some(),
        keys: kb.unwrap_or(RECT_EMPTY),
        zoom_ok: graph && a.settings.pinch_zoom,
        ..TgView::default()
    }
}

pub fn library_event(a: &mut App, ev: &InEvent) {
    // (a contact or a gesture on the main menu: it is in use, and comes back from a notebook where it was left,
    // not at home)
    if a.scr == ScreenId::Library && (app_pointer_phase(ev) == 0 || ev.type_ == InType::Gesture) {
        lib(a).home_pending = false;
    }
    library_event_in(a, ev);
    // (a keyboard opened or closed; the editor pushes its own as it enters)
    if a.scr == ScreenId::Library {
        input_touch_context(library_touch_view(a));
    }
}

fn library_event_in(a: &mut App, ev: &InEvent) {
    if library_record_active(a) && record_event(a, ev) {
        return;
    }
    if files_in_front(a) {
        files_event(a, ev);
        return;
    }
    if graph_shown(a) {
        graph_event(a, ev);
        return;
    }
    if ev.type_ == InType::Gesture {
        library_gesture(a, ev);
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
        if r == TsResult::FromFile {
            files_open_picture(a, crate::bgwork_copy::CopyTo::Templates);
        } else if r == TsResult::Picked || r == TsResult::Back || r == TsResult::Cancel {
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
        let r = if touch_cancelled(ev) {
            dispmenu_cancel(&mut dm, a, &mut d)
        } else {
            dispmenu_pointer(&mut dm, a, ph, ev.x, ev.y, ev.t_ms, &mut d)
        };
        lib(a).dm = dm;
        if r == DmResult::ClosedAll {
            close_settings(a, RefreshMode::Ui); // the X closes all of Settings
        } else if r == DmResult::Closed {
            lib(a).modal = Modal::Settings; // back to the settings sheet it was opened from
            redraw(a, RefreshMode::Ui);
        } else if r == DmResult::Inverted {
            redraw(a, RefreshMode::Full);
        } else if r == DmResult::Refit {
            // (the editor, not open here, lays its page out again when it is next entered; the row shows the
            // new setting, with no toast, and one left up from before goes)
            editor_fit_changed(a, false);
            redraw(a, RefreshMode::Ui);
            app_toast_dismiss(a);
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
    if modal == Modal::Wifi && wifi_sheet_scroll(a, ph, ev) {
        return;
    }
    if ph == 0 {
        // The question before a notebook is deleted takes its Delete once it has been shown, never before
        // (app_asked_seen): a contact that came down on that row sooner answers nothing, down to its lift. Keep it
        // moves nothing, and answers at once.
        let unseen = modal == Modal::Delete && !app_asked_seen(a, ev);
        let s = lib(a);
        press_begin(&mut s.press, ev.x, ev.y, ev.t_ms);
        s.pressed_at = now_ms();
        let h = hits_find(&s.hits, ev.x, ev.y).copied().filter(|h| !(unseen && h.id == H_DELETE_YES));
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
    crate::userfonts::userfonts_tick(a, now); // (a font check that ended with no verdict is queued again after its wait)
    if files_shown(a) {
        // (not under Exit Grimoire?, asked at a home inside Files: a draw of Files' own would paint over it)
        if files_in_front(a) {
            files_tick(a, now);
        }
        return;
    }
    if lib(a).modal == Modal::Font && !graph_shown(a) {
        font_sheet_tick(a, now);
    }
    if graph_shown(a) {
        graph_tick(a, now);
        return;
    }
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
    // The Pair sheet's status line names what is connected.
    let s = a.lib.as_deref().unwrap();
    if s.modal == Modal::PairMenu && s.shown_hid_changes != a.hid_changes {
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
    if lib(a).modal == Modal::Wifi {
        wifi_sheet_tick(a, now);
    }
    upd_poll(a);
    let s = a.lib.as_deref().unwrap();
    if s.modal == Modal::PairMenu && s.shown_bt_changes != a.bt_changes {
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
        && now.wrapping_sub(s.pressed_at) >= HOLD_MS
    {
        let row = s.pressed.map_or(0, |p| p.arg);
        s.pressed = None;
        forget_row(a, row);
    }
    let s = lib(a);
    if s.modal == Modal::Gestures && s.gestures_group >= 0 && s.record_action == SHORTCUT_NONE
        && !s.gestures_reset_confirm && s.pressed.is_some_and(|p| p.id == H_GESTURE_ROW)
        && !s.press.moved && now.wrapping_sub(s.pressed_at) >= HOLD_MS
    {
        let row = s.pressed.take().unwrap().arg;
        if let Some(row) = binding_row_at(GESTURES, s.gestures_group as usize, row as usize) {
            (GESTURES.forget)(a, row);
            redraw(a, RefreshMode::Ui);
        }
    }
    // A card held still (HOLD_MS, 2.2): its menu, as its "..." opens it; the lift after it does nothing (nothing is
    // pressed any more, and the menu's drawing took the pressed look away).
    let on_library = a.scr == ScreenId::Library;
    let s = lib(a);
    if on_library
        && s.modal == Modal::None
        && s.pressed.is_some_and(|p| p.id == H_CARD)
        && !s.press.moved
        && now.wrapping_sub(s.pressed_at) >= HOLD_MS
    {
        let arg = s.pressed.take().map_or(0, |p| p.arg);
        act(a, &Hit { id: H_CARD_MORE, arg, ..Default::default() });
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
    if it.is_files {
        // the Files card (2.2): "files", its name "Files", no picture
        let t = if name { tr("Files") } else if luma { "none" } else { "files" };
        csnprintf!(out, cap, "%s", t);
        return 0;
    }
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
    static SHEETS: [&str; 19] = [
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
        "about",       // (What's new)
    ];
    if a.lib.is_none() {
        return -1;
    }
    // The file picker (2.1.0-beta.2): the files.* keys (docs/SIM.md).
    if let Some(k) = key.strip_prefix("files.") {
        let h = a.h;
        let mut text = String::new();
        if !lib(a).files.probe(h, k, &mut text) {
            return -1;
        }
        csnprintf!(out, cap, "%s", &text);
        return 0;
    }
    // The Graph opened from the Library (2.0.0-beta.2): the editor's map.* keys (docs/SIM.md), map.picker its
    // notebooks' browser (past the caps), map.keep 1 while a notebook it opened is in front; graph.omitted the
    // Library's notebooks the Graph of all notebooks leaves out (by their own setting or a folder's).
    if key == "map" || key.starts_with("map.") {
        let s = lib(a);
        match key {
            "map.picker" => {
                csnprintf!(out, cap, "%d", s.map_picker.open as i32);
            }
            "map.keep" => {
                csnprintf!(out, cap, "%d", s.map_keep.is_some() as i32);
            }
            _ => {
                let mut text = String::new();
                if !mapview_probe(&s.map, key.strip_prefix("map.").unwrap_or(""), &mut text) {
                    return -1;
                }
                csnprintf!(out, cap, "%s", &text);
            }
        }
        return 0;
    }
    if key == "graph.omitted" {
        let mut ids: Vec<String> = Vec::new();
        let mut books = Vec::new();
        store_list(&mut books);
        for nb in &books {
            if libindex_omits(&graph_book_of(nb, &Nav::default())) {
                ids.push(nb.id.as_str().to_string());
            }
        }
        ids.sort();
        csnprintf!(out, cap, "%s", &ids.join(" "));
        return 0;
    }
    if key == "shortcuts.device" || key == "shortcuts.own" {
        // The Shortcuts sheet's device (its profile's slug, "" for none), and its own shortcuts: "on", "off"
        // (the file kept) or "none" (no file).
        let dev = shortcut_dev_now(a);
        let own = dev.as_ref().and_then(|d| a.shortcut_profiles.iter().find(|p| p.slug == d.slug));
        let text = if key == "shortcuts.device" {
            dev.map_or(String::new(), |d| d.slug)
        } else {
            own.map_or("none", |p| if p.on { "on" } else { "off" }).to_string()
        };
        csnprintf!(out, cap, "%s", &text);
        return 0;
    }
    let pp = per_page(a);
    let s = lib(a);
    if key == "exit" {
        // Exit Grimoire?'s highlighted answer ("yes" or "no"), "" while it is not up.
        csnprintf!(out, cap, "%s", if s.modal != Modal::Exit { "" } else if s.exit_no { "no" } else { "yes" });
        return 0;
    }
    if key == "sheet" && s.modal == Modal::Exit {
        csnprintf!(out, cap, "exit");
    } else if key == "sheet" && s.modal == Modal::PairMenu {
        csnprintf!(out, cap, "pair-menu");
    } else if key == "sheet" && s.modal == Modal::Wifi {
        csnprintf!(out, cap, "wifi");
    } else if key == "sheet" && s.modal == Modal::Merge {
        csnprintf!(out, cap, "merge");
    } else if key == "sheet" && s.modal == Modal::Font {
        csnprintf!(out, cap, "font");
    } else if key == "font.pick" {
        // The Font sheet's font highlighted, by its settings.ini value (a family's id, or file:<path> for one of
        // the owner's own); "" while the sheet is not up.
        if s.modal != Modal::Font {
            csnprintf!(out, cap, "%s", "");
        } else if s.font_file.is_empty() {
            csnprintf!(out, cap, "%s", FONT_FAMILIES[s.font_pick.min(FONT_FAMILIES.len() - 1)].id);
        } else {
            csnprintf!(out, cap, "file:%s", &s.font_file);
        }
    } else if key == "font.rows" {
        // The Font sheet's own fonts: how many files of grimoire/fonts it lists.
        csnprintf!(out, cap, "%d", s.fonts.len() as i32);
    } else if key == "font.checking" {
        csnprintf!(out, cap, "%d", crate::userfonts::userfonts_checking(&s.fonts) as i32); // of them, still being checked
    } else if key == "font.shown" {
        // The rows of own fonts on the page shown, by their paths, joined by "|"; font.page is "2/3".
        let v: Vec<&str> = s.font_own_rows.iter().filter_map(|(i, _)| s.fonts.get(*i)).map(|f| f.rel.as_str()).collect();
        csnprintf!(out, cap, "%s", &v.join("|"));
    } else if key == "font.page" {
        let per = s.font_per_page.max(1);
        csnprintf!(out, cap, "%d/%d", (s.font_first / per + 1) as i32, s.fonts.len().div_ceil(per).max(1) as i32);
    } else if key == "font.sample" {
        // How the sample card was drawn: "family", "own" (by the owner's font itself) or "preview" (by the
        // picture the check made); "" while the sheet is not up.
        csnprintf!(out, cap, "%s", if s.modal == Modal::Font { s.font_sample_how } else { "" });
    } else if let Some(rel) = key.strip_prefix("font.state.") {
        // An own font's row by its path: "ready", "checking", "unreadable", "toobig", "nolatin", "format",
        // "slow" or "missing"; "" for a file the sheet does not list. font.note.<path> is what the row says.
        let mut f = s.fonts.iter().find(|f| f.rel == rel).cloned();
        if let Some(f) = f.as_mut() {
            crate::userfonts::userfont_resolve(f);
        }
        csnprintf!(out, cap, "%s", f.as_ref().map_or("", |f| f.state_name()));
    } else if let Some(rel) = key.strip_prefix("font.note.") {
        let mut f = s.fonts.iter().find(|f| f.rel == rel).cloned();
        if let Some(f) = f.as_mut() {
            crate::userfonts::userfont_resolve(f);
        }
        csnprintf!(out, cap, "%s", &f.as_ref().map_or(String::new(), font_row_note));
    } else if key == "sheet" && s.modal == Modal::SideButton {
        csnprintf!(out, cap, "pen-button");
    } else if key == "sheet" && s.modal == Modal::EraserMode {
        csnprintf!(out, cap, "eraser-mode");
    } else if key == "sheet" && s.modal == Modal::DeviceInfo {
        csnprintf!(out, cap, "about-device");
    } else if key == "sheet" && s.modal == Modal::LinkQr {
        csnprintf!(out, cap, "about-link");
    } else if key == "sheet" && s.modal == Modal::FilesMore {
        csnprintf!(out, cap, "files-more");
    } else if key == "sheet" {
        match SHEETS.get(s.modal as usize) {
            Some(m) => csnprintf!(out, cap, "%s", *m),
            None => csnprintf!(out, cap, "%d", s.modal as i32),
        };
    } else if let Some(k) = key.strip_prefix("about.") {
        // What's new (docs/SIM.md): its view; its page (from 1) and pages; 1 on Read all's sheet (the list
        // or a version's notes); the list's rows on its page, the row marked as this version's, each
        // row's versions and date; the notes shown, their body size, their leading and whether the page
        // shown has the signature; the card's notes: the blocks it shows, of how many, and 1 when the
        // last shown is cut.
        let open = s.modal == Modal::About;
        let view = if open { Some(s.about_view) } else { None };
        let all = crate::about::notes();
        let row = |n: &str| -> Option<&crate::about::Entry> {
            let n = n.parse::<usize>().ok()?.checked_sub(1)?;
            all.get(*s.about_rows.get(n).filter(|_| view == Some(AboutView::List))?)
        };
        if k == "view" {
            let v = match view {
                Some(AboutView::Card) => "card",
                Some(AboutView::List) => "list",
                Some(AboutView::Note) => "note",
                None => "",
            };
            csnprintf!(out, cap, "%s", v);
            return 0;
        } else if k == "note" {
            let e = match view {
                Some(AboutView::Card) => all.get(crate::about::current(all, GRIMOIRE_VERSION)),
                Some(AboutView::Note) => all.get(s.about_note),
                _ => None,
            };
            csnprintf!(out, cap, "%s", &e.map_or(String::new(), |e| e.label()));
            return 0;
        } else if let Some(n) = k.strip_prefix("row.") {
            csnprintf!(out, cap, "%s", &row(n).map_or(String::new(), |e| e.label()));
            return 0;
        } else if let Some(n) = k.strip_prefix("row_date.") {
            csnprintf!(out, cap, "%s", row(n).map_or("", |e| crate::about::date_label(e.date)));
            return 0;
        } else if k == "body_px" {
            // (in px to a tenth, "44" or "38.5"; 0 but on a version's notes)
            let px = if view == Some(AboutView::Note) { s.about_body_px } else { 0.0 };
            let t = format!("{px:.1}");
            csnprintf!(out, cap, "%s", t.strip_suffix(".0").unwrap_or(&t));
            return 0;
        } else if k == "lead" {
            // (to a hundredth, "1.3" or "1.22"; 0 but on a version's notes)
            let lead = if view == Some(AboutView::Note) { s.about_lead } else { 0.0 };
            let t = format!("{lead:.2}");
            let t = t.trim_end_matches('0').trim_end_matches('.');
            csnprintf!(out, cap, "%s", if t.is_empty() { "0" } else { t });
            return 0;
        }
        let card = view == Some(AboutView::Card);
        let list = view == Some(AboutView::List);
        let v = match k {
            "page" if open => if list { s.about_list_page + 1 } else { s.about_page + 1 },
            "pages" if open => s.about_pages,
            "full" if open => !card as i32,
            "entries" if open => all.len() as i32,
            "rows" if list => s.about_rows.len() as i32,
            "marked" if list => s.about_marked,
            "signature" if open && !list => s.about_signed as i32,
            "card_blocks" if card => s.about_card.0,
            "card_notes" if card => s.about_card.1,
            "card_cut" if card => s.about_card.2 as i32,
            "page" | "pages" | "full" | "entries" | "rows" | "marked" | "signature" | "card_blocks" | "card_notes"
            | "card_cut" => 0,
            _ => return -1,
        };
        csnprintf!(out, cap, "%d", v);
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
                H_MORE_OPEN=>"more.open",H_MORE_RENAME=>"more.rename",H_MORE_PDF=>"more.pdf",H_MORE_DELETE=>"more.delete",
                H_MORE_GRAPH=>"more.graph",H_FMORE_GRAPH=>"fmore.graph",H_DELETE_YES=>"delete.yes",
                H_SET_THEME=>"theme",H_NEW_CREATE=>"create",H_MORE_MOVE=>"more.move",
                H_FMORE_RENAME=>"fmore.rename",H_FMORE_MOVE=>"fmore.move",H_FMORE_REMOVE=>"fmore.remove",
                H_FMORE_HOME => "fmore.home",
                H_MOVE_TOP=>"move.top",H_MOVE_TO=>"move.to.",H_MOVE_NEW=>"move.new",H_MOVE_PREV=>"move.prev",H_MOVE_NEXT=>"move.next",
                H_SET_PENS => "pen-feel", H_SET_CALIB => "calibrate", H_SET_DISPLAY => "display",
                H_SET_SLEEP => "sleep-clock", H_SET_CLOCK => "clock-settings",
                H_KOREADER => "ko", H_SHORTCUTS => "shortcuts", H_PAIR_MENU => "pair-menu", H_GESTURES => "gestures",
                H_SHORTCUT_DEVICE => "shortcuts.device", H_SHORTCUT_OWN => "shortcuts.own", H_SHORTCUT_RELOAD => "shortcuts.reload",
                H_UPDATES => "updates", H_CANCEL => "done", H_GESTURES_MASTER => "gestures.master",
                H_UPD_CHANNEL => "updates.channel", H_UPD_NOW => "updates.now",
                H_SET_DARK => "dark",
                H_SET_ROTATE => "rotate", H_SHORTCUT_ENABLE => "shortcuts.enable", H_BT_TOGGLE => "bluetooth",
                H_PAIR => "pair", H_BUTTONS => "buttons",
                H_GESTURES_RESET => "gestures.reset", H_GESTURES_DONE => "done",
                H_GESTURES_RESET_CONFIRM => "gestures.reset.confirm",
                H_GESTURES_RESET_CANCEL => "gestures.reset.cancel",
                H_RECORD_USE => "record.use", H_RECORD_RETRY => "record.retry", H_RECORD_CANCEL => "record.cancel",
                H_RECORD_DISABLE => "record.disable",
                H_GESTURE_GROUP => "gestures.group.", H_GESTURE_ROW => "gestures.row.",
                H_BUTTON_GROUP => "buttons.group.", H_BUTTON_ROW => "buttons.row.",
                H_BUTTONS_DONE => "done", H_BUTTONS_RESET => "buttons.reset",
                H_PAPER => "paper.", H_PAPER_CUSTOM => "paper.custom", H_PAPER_MORE => "paper.more", H_NEW_RENAME => "new.rename", H_NEW_FILE => "new.file",
                H_EXIT_YES => "exit.yes", H_EXIT_NO => "exit.no",
                H_MERGE_INTO => "merge.into", H_MERGE_BOTH => "merge.both",
                H_FILES_OPEN => "files-card.open", H_FILES_HIDE => "files-card.hide",
                H_BACK => "back", H_ABOUT => "about", H_ABOUT_CLOSE => "about.close",
                H_ABOUT_PREV => "about.prev", H_ABOUT_NEXT => "about.next", H_ABOUT_MORE => "about.more",
                H_ABOUT_ROW => "about.row.", H_ABOUT_BACK => "about.back",
                H_LINK => "about.link.", H_LINK_PAGE => if hit.arg < 0 { "about.link.prev" } else { "about.link.next" },
                H_SET_FONT => "font", H_FONT_PICK => "font.", H_FONT_DONE => "font.done", H_FONT_CANCEL => "font.cancel",
                H_FONT_OWN => "font.file:", H_FONT_PAGE => if hit.arg < 0 { "font.prev" } else { "font.next" },
                H_SET_CAT => "settings.cat.",
                H_SET_CAT_PAGE => if hit.arg < 0 { "settings.cat.prev" } else { "settings.cat.next" },
                H_SET_ROWS_PAGE => if hit.arg < 0 { "settings.row.prev" } else { "settings.row.next" },
                H_SETTINGS_PAGE => if hit.arg < 0 { "settings.page.prev" } else { "settings.page.next" },
                H_WIFI_MENU => "wifi", H_WIFI_RADIO => "wifi.radio", H_WIFI_NET => "wifi.net.",
                H_WIFI_TAB => "wifi.tab", H_WIFI_SCAN => "wifi.scan", H_WIFI_NEAR => "wifi.near.",
                H_WIFI_DELETE => "wifi.delete.", H_WIFI_OTHER => "wifi.other",
                H_WIFI_OTHER_OPEN => "wifi.other.open", H_WIFI_OTHER_WPA => "wifi.other.wpa",
                H_WIFI_OTHER_WPA3 => "wifi.other.wpa3", H_WIFI_OTHER_EAP => "wifi.other.eap",
                H_WIFI_OTHER_CANCEL => "wifi.other.cancel",
                // (the new Settings page's rows and submenus that had no name: its Back and X, and the
                // choice lists that open from a row)
                H_SETTINGS_BACK => "settings.back", H_SETTINGS_CLOSE => "settings.close",
                H_SET_LANGUAGE => "language", H_LANG_PICK => "language.", H_LANG_BACK => "language.back",
                H_SET_SIDE => "pen-button", H_SIDE_PICK => "pen-button.", H_SIDE_BACK => "pen-button.back",
                H_SET_ERASER => "eraser-mode", H_ERASER_PICK => "eraser-mode.", H_ERASER_BACK => "eraser-mode.back",
                H_SET_PINCH => "pinch", H_SET_MIRROR => "mirror",
                H_SET_DEVICE => "about-device", H_DEVICE_BACK => "about-device.back",
                _ => "",
            });
            if hit.id == H_LANG_PICK {
                name.push_str(i18n_language_code(hit.arg).unwrap_or("auto"));
            }
            if matches!(hit.id, H_SIDE_PICK | H_ERASER_PICK) { name.push_str(&hit.arg.to_string()); }
            if hit.id == H_FONT_PICK {
                name.push_str(FONT_FAMILIES.get(hit.arg as usize).map_or("", |f| f.id));
            }
            if hit.id == H_FONT_OWN {
                name.push_str(s.fonts.get(hit.arg as usize).map_or("", |f| f.rel.as_str()));
            }
            if hit.id == H_ABOUT_ROW {
                name.push_str(crate::about::notes().get(hit.arg as usize).map_or("", |e| e.versions[0]));
            }
            if hit.id == H_LINK { name.push_str(&hit.arg.to_string()); }
            if hit.id == H_PAPER { name.push_str(&hit.arg.to_string()); }
            if hit.id == H_WIFI_NET { name.push_str(&(hit.arg + 1).to_string()); }
            if hit.id == H_WIFI_NEAR { name.push_str(&(hit.arg + 1).to_string()); }
            if hit.id == H_SET_CAT { name.push_str(&hit.arg.to_string()); }
            if hit.id == H_WIFI_TAB { name.push_str(if hit.arg == 0 { ".near" } else { ".saved" }); }
            if hit.id == H_WIFI_DELETE { name.push_str(&(hit.arg + 1).to_string()); }
            if hit.id == H_MOVE_TO { name.push_str(s.folders.get(hit.arg as usize).map_or("", |f| f.as_str())); }
            if matches!(hit.id, H_GESTURE_GROUP | H_BUTTON_GROUP) {
                if let Some(group) = ACTION_GROUPS.get(hit.arg as usize) { name.push_str(group.probe); }
            } else if matches!(hit.id, H_GESTURE_ROW | H_BUTTON_ROW) {
                name.push_str(&(hit.arg + 1).to_string());
            }
        }
        csnprintf!(out, cap, "%s", &name);
    } else if let Some(name) = key.strip_prefix("ui_has.") {
        // 1 when a control of that name (as ui_at names them) is on the Library or its sheet now, else 0.
        let found = library_control_point(a, name).is_some();
        csnprintf!(out, cap, "%d", found as i32);
    } else if key == "learning" {
        // The action the Buttons sheet waits for a press for ("undo", "next-page"), else empty.
        let n = if s.modal == Modal::Buttons && s.buttons_group >= 0 && s.learn_row >= 0 {
            shortcut_action_name(learning_action(s))
        } else {
            None
        };
        csnprintf!(out, cap, "%s", n.unwrap_or(""));
    } else if key == "buttons.dev" {
        // The device whose own buttons the Buttons sheet shows and learns into, by its profile's slug; empty for
        // the default (controller.ini).
        let slug = buttons_profile(a).map_or(String::new(), |i| a.shortcut_profiles[i].slug.clone());
        csnprintf!(out, cap, "%s", &slug);
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
    } else if key == "home.pending" {
        // 1 while the main menu has not been used in this run (it opens at home as it comes in front, 2.2)
        csnprintf!(out, cap, "%d", s.home_pending as i32);
    } else if key == "folders" {
        csnprintf!(out, cap, "%s", &s.folders.join(", "));
    } else if key == "mirror" {
        // The mirror PDFs by directory: "." (the top) and each directory under it, sorted, with how many
        // PDFs it holds ("School 0" for one left empty).
        let count = |dir: &std::path::Path| {
            std::fs::read_dir(dir).map_or(0, |d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".pdf")).count())
        };
        let root = std::path::PathBuf::from(app_mirror_dir());
        let mut dirs: Vec<String> = std::fs::read_dir(&root)
            .map(|d| d.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        dirs.sort();
        let mut text = format!(". {}", count(&root));
        for d in dirs {
            text.push_str(&format!(", {} {}", d, count(&root.join(&d))));
        }
        csnprintf!(out, cap, "%s", &text);
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
    } else if key == "caret" {
        // The text with its caret (and selection) drawn in, keyboard_caret_probe; empty with no keyboard.
        let shown = if s.modal == Modal::Kb { keyboard_caret_probe(&s.kb) } else { String::new() };
        csnprintf!(out, cap, "%s", &shown);
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

    // Every control id (H_*) is its own: two changes once both took 78, and the Updates sheet's switch
    // answered as a notebook menu's Graph row.
    #[test]
    fn control_ids_are_distinct() {
        let src = include_str!("library.rs");
        let mut seen: Vec<(i64, &str)> = Vec::new();
        for line in src.lines() {
            let Some(rest) = line.strip_prefix("const H_") else { continue };
            let Some((name, value)) = rest.split_once(": i32 = ") else { continue };
            let Ok(v) = value.trim_end_matches(';').trim().parse::<i64>() else { continue };
            if let Some((_, other)) = seen.iter().find(|(w, _)| *w == v) {
                panic!("H_{name} and H_{other} are both {v}");
            }
            seen.push((v, name));
        }
        assert!(seen.len() > 50);
    }

    // A sheet past the C's (0 to 18) takes a number from 300 on, never the next free number after the C's;
    // each number is its own.
    #[test]
    fn sheet_numbers_past_the_c_ones_are_lane_blocks() {
        let src = include_str!("library.rs");
        let body = src.split_once("enum Modal {").unwrap().1.split_once("\n}").unwrap().0;
        let mut seen: Vec<i64> = Vec::new();
        for line in body.lines() {
            let Some((name, value)) = line.split_once(" = ") else { continue };
            let Ok(v) = value.split(',').next().unwrap().trim().parse::<i64>() else { continue };
            assert!(v <= 18 || (300..400).contains(&v), "Modal::{} = {v}", name.trim());
            assert!(!seen.contains(&v), "Modal = {v} twice");
            seen.push(v);
        }
        assert!(seen.len() >= 20 && seen.contains(&300));
    }

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

    // Ctrl+S in the Library writes what is waiting (the settings), with no toast since the owner's toast pass
    // (2026-11-04): a setting changed in memory is in settings.ini after it.
    #[test]
    fn ctrl_s_in_the_library_writes_the_settings_without_a_toast() {
        let root = std::env::temp_dir().join(format!("grimoire-lib-save-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::None;
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        a.settings.mirror_pdf = !a.settings.mirror_pdf; // (in memory only, so far)
        assert!(library_shortcut(&mut a, SHORTCUT_SAVE));
        let mut on_disk = Settings::default();
        settings_load(&mut on_disk);
        assert_eq!(on_disk.mirror_pdf, a.settings.mirror_pdf);
        assert!(!a.toast_visible);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The Library's reload reads each notebook.ini once: its cards and its folders come from the one listing
    // (browser.md 9: it read every notebook.ini twice, 1,000 opens on FUSE for 500 notebooks at each enter).
    #[test]
    fn a_reload_reads_each_notebook_once() {
        let root = std::env::temp_dir().join(format!("grimoire-reload-once-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        for (name, folder) in [("A", ""), ("B", "School"), ("C", "School/2025"), ("D", "Work")] {
            let mut nb = NotebookInfo::default();
            assert_eq!(store_create(name, folder, PAPER_BLANK, &mut nb), 0);
        }
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::None;
        let before = store_info_reads();
        reload(&mut a);
        assert_eq!(store_info_reads() - before, 4, "one read of each notebook.ini");
        let s = lib(&mut a);
        assert_eq!(s.books.len(), 4);
        assert_eq!(s.folders, ["School", "School/2025", "Work"]);
        // (the same folders as the listing of its own)
        let mut folders = Vec::new();
        assert_eq!(store_list_folders(&mut folders), 0);
        assert_eq!(lib(&mut a).folders, folders);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The Files card's second line, the storage's free space (2.2), is read when the Library reloads, not at each
    // draw of the card: drawing the Library three times asks for it no more.
    #[test]
    fn the_files_cards_free_space_is_read_on_reload_not_on_each_draw() {
        let root = std::env::temp_dir().join(format!("grimoire-files-free-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::env::set_var("GRIMOIRE_STORAGE", &root);
        assert_eq!(store_init(root.join("grimoire").to_str().unwrap()), 0);
        let mut a = pen_record_app();
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        a.settings.files_card = true;
        lib(&mut a).modal = Modal::None;
        let reads = || crate::browser::FREE_LINE_READS.with(|n| n.get());
        let r0 = reads();
        reload(&mut a);
        assert_eq!(reads(), r0 + 1);
        assert!(lib(&mut a).files_free.as_deref().is_some_and(|l| l.starts_with("Free: ")));
        for _ in 0..3 {
            redraw(&mut a, RefreshMode::Ui);
        }
        assert!(lib(&mut a).items[0].is_files && lib(&mut a).hits.h.iter().any(|h| h.id == H_CARD), "the card was drawn");
        assert_eq!(reads(), r0 + 1, "no draw asked for it again");
        library_free(&mut a);
        canvas_free(&mut a.screen);
        std::env::remove_var("GRIMOIRE_STORAGE");
        let _ = std::fs::remove_dir_all(&root);
    }

    // The Files card (2.2): not there by default; with files_card it is the top level's last card, after every
    // folder and notebook, and in no folder. Its "..." opens its menu, whose Hide from Library takes it away and
    // writes files_card back out (the line goes).
    #[test]
    fn the_files_card_is_the_top_levels_last_card() {
        let root = std::env::temp_dir().join(format!("grimoire-files-card-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        for (name, folder) in [("A", ""), ("B", "School"), ("C", "")] {
            let mut nb = NotebookInfo::default();
            assert_eq!(store_create(name, folder, PAPER_BLANK, &mut nb), 0);
        }
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::None;
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        reload(&mut a);
        assert!(lib(&mut a).items.iter().all(|it| !it.is_files));
        a.settings.files_card = true;
        reload(&mut a);
        let s = lib(&mut a);
        assert_eq!(s.items.len(), 4, "School, A, C and Files");
        assert!(s.items[0].is_folder && s.items[3].is_files);
        assert_eq!(s.items.iter().filter(|it| it.is_files).count(), 1);
        s.cur_folder.set("School");
        reload(&mut a);
        assert!(lib(&mut a).items.iter().all(|it| !it.is_files));
        lib(&mut a).cur_folder.clear();
        reload(&mut a);
        redraw(&mut a, RefreshMode::Ui);
        let mut out = String::new();
        assert_eq!(library_probe(&mut a, "card.4", &mut out, 64), 0);
        assert_eq!(out, "files");
        act(&mut a, &Hit { id: H_CARD_MORE, arg: 3, ..Default::default() });
        assert_eq!(lib(&mut a).modal, Modal::FilesMore);
        act(&mut a, &Hit { id: H_FILES_HIDE, ..Default::default() });
        assert!(!a.settings.files_card);
        assert_eq!(lib(&mut a).modal, Modal::None);
        assert!(lib(&mut a).items.iter().all(|it| !it.is_files));
        let mut on_disk = Settings::default();
        settings_load(&mut on_disk);
        assert!(!on_disk.files_card);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The question before a notebook is deleted takes its Delete only once it has been shown (issue #144: the guard
    // Files' own question has, app_asked_seen): a contact that came down on that row before the question's frame
    // went to the panel, or sooner than PEN_REOPEN_MS after it, deletes nothing and leaves the question up. Keep it
    // moves nothing and answers at once, as before; a tap on its Delete after that deletes, as before.
    #[test]
    fn a_notebooks_delete_question_takes_its_delete_once_shown() {
        let root = std::env::temp_dir().join(format!("grimoire-delete-seen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("A", "", PAPER_BLANK, &mut nb), 0);
        let mut a = pen_record_app();
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        lib(&mut a).modal = Modal::None;
        reload(&mut a);
        redraw(&mut a, RefreshMode::Ui);
        let centre = |a: &mut App, id: i32| {
            let h = lib(a).hits.h.iter().find(|h| h.id == id).copied().unwrap_or_else(|| panic!("no control {id}"));
            ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32)
        };
        let tap = |a: &mut App, (x, y): (f32, f32), t_ms: u32, k_ms: u32| {
            for type_ in [InType::TouchDown, InType::TouchUp] {
                library_event_in(a, &InEvent { type_, x, y, t_ms, k_ms, ..Default::default() });
            }
        };
        let ask = |a: &mut App| {
            act(a, &Hit { id: H_CARD_MORE, arg: 0, ..Default::default() });
            assert_eq!(lib(a).modal, Modal::More);
            let p = centre(a, H_MORE_DELETE);
            tap(a, p, 1000, 0);
            assert_eq!((lib(a).modal, a.asked), (Modal::Delete, Asked::Drawn));
        };
        ask(&mut a);
        let yes = centre(&mut a, H_DELETE_YES);
        for t in [1000, now_ms().wrapping_add(60_000)] {
            tap(&mut a, yes, t, 0);
            assert!(lib(&mut a).modal == Modal::Delete && lib(&mut a).books.len() == 1 && lib(&mut a).pressed.is_none(), "before its frame");
        }
        app_flush(&mut a);
        let Asked::Shown(at) = a.asked else { panic!("{:?}", a.asked) };
        for (t, k) in [(at, 0), (at.wrapping_add(PEN_REOPEN_MS - 1), 0), (at.wrapping_add(9_000), at.wrapping_sub(5))] {
            tap(&mut a, yes, t, k);
            assert!(lib(&mut a).modal == Modal::Delete && lib(&mut a).books.len() == 1, "too soon after its frame: {t} (kernel {k}), shown at {at}");
        }
        // Keep it at once
        let keep = centre(&mut a, H_CANCEL);
        tap(&mut a, keep, at, 0);
        assert!(lib(&mut a).modal == Modal::None && lib(&mut a).books.len() == 1);
        // asked again: Keep it even before the frame, as before
        ask(&mut a);
        let keep = centre(&mut a, H_CANCEL);
        tap(&mut a, keep, 1000, 0);
        assert!(lib(&mut a).modal == Modal::None && lib(&mut a).books.len() == 1);
        // and its Delete, once shown
        ask(&mut a);
        app_flush(&mut a);
        let Asked::Shown(at) = a.asked else { panic!("{:?}", a.asked) };
        let yes = centre(&mut a, H_DELETE_YES);
        tap(&mut a, yes, at.wrapping_add(PEN_REOPEN_MS), 0);
        assert!(lib(&mut a).modal == Modal::None && lib(&mut a).books.is_empty(), "its Delete, once shown");
        assert_eq!(a.toast.as_str(), "Moved to grimoire/trash");
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A card held still for HOLD_MS (2.2) opens its menu as its "..." does: a notebook's, a folder's, the Files
    // card's; a press that moved, or one let go sooner, does not.
    #[test]
    fn a_card_held_opens_its_menu() {
        let root = std::env::temp_dir().join(format!("grimoire-card-hold-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        for (name, folder) in [("A", ""), ("B", "School")] {
            let mut nb = NotebookInfo::default();
            assert_eq!(store_create(name, folder, PAPER_BLANK, &mut nb), 0);
        }
        let mut a = pen_record_app();
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        a.settings.files_card = true;
        lib(&mut a).modal = Modal::None;
        reload(&mut a);
        redraw(&mut a, RefreshMode::Ui);
        let hold = |a: &mut App, idx: i32, moved: bool, ms: u32| {
            let s = lib(a);
            s.pressed = s.hits.h.iter().find(|h| h.id == H_CARD && h.arg == idx).copied();
            assert!(s.pressed.is_some(), "card {idx} drawn");
            s.press = Press { moved, ..Default::default() };
            s.pressed_at = 1000;
            library_tick(a, 1000 + ms);
        };
        for (idx, want) in [(0, Modal::FolderMore), (1, Modal::More), (2, Modal::FilesMore)] {
            hold(&mut a, idx, false, HOLD_MS - 1);
            assert_eq!(lib(&mut a).modal, Modal::None, "not yet");
            hold(&mut a, idx, true, HOLD_MS + 5);
            assert_eq!(lib(&mut a).modal, Modal::None, "it moved");
            hold(&mut a, idx, false, HOLD_MS + 5);
            assert_eq!(lib(&mut a).modal, want, "card {idx}");
            assert!(lib(&mut a).pressed.is_none() && lib(&mut a).target == idx);
            lib(&mut a).modal = Modal::None;
            redraw(&mut a, RefreshMode::Ui);
        }
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A gesture bound to files or home (gestures.ini) acts while Files is up, as its key does: home leaves Files for
    // the Library's top, files goes to Files' root. And the Files card is no menu's notebook: a menu's target that
    // lands on it reads as none.
    #[test]
    fn files_gestures_act_in_files_and_the_card_is_no_target() {
        let root = std::env::temp_dir().join(format!("grimoire-files-gestures-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("documents")).unwrap();
        std::env::set_var("GRIMOIRE_STORAGE", &root);
        assert_eq!(store_init(root.join("grimoire").to_str().unwrap()), 0);
        let mut a = pen_record_app();
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        a.settings.files_card = true;
        lib(&mut a).modal = Modal::None;
        reload(&mut a);
        let three = |kind| GestureKey::Touch { fingers: 3, kind };
        assert!(gesturemap_set(&mut a.gestures, three(TouchKind::Tap), SHORTCUT_HOME) >= 0);
        assert!(gesturemap_set(&mut a.gestures, three(TouchKind::Up), SHORTCUT_FILES) >= 0);
        let gesture = |a: &mut App, id: i32| library_event_in(a, &InEvent { type_: InType::Gesture, value: id, ..Default::default() });
        assert!(library_shortcut(&mut a, SHORTCUT_FILES) && files_shown(&a));
        assert!(lib(&mut a).files.go(crate::fsview::StoragePath::new(b"documents").unwrap()));
        gesture(&mut a, G_3_UP);
        let (mut out, h) = (String::new(), a.h);
        assert!(lib(&mut a).files.probe(h, "place", &mut out) && out == "root", "files: Files' root");
        gesture(&mut a, G_3_TAP);
        assert!(!files_shown(&a), "home: out of Files");
        let s = lib(&mut a);
        s.target = s.items.iter().position(|it| it.is_files).unwrap() as i32;
        assert!(target_item(s).is_none());
        library_free(&mut a);
        canvas_free(&mut a.screen);
        std::env::remove_var("GRIMOIRE_STORAGE");
        let _ = std::fs::remove_dir_all(&root);
    }

    // Files' two actions (2.2, no default key): files opens Files in place mode whether or not its card is in the
    // Library, and inside it goes back to its root; home leaves Files and any folder for the Library's top.
    #[test]
    fn the_files_and_home_actions() {
        let root = std::env::temp_dir().join(format!("grimoire-files-actions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let g = root.join("grimoire");
        std::fs::create_dir_all(root.join("documents")).unwrap();
        std::env::set_var("GRIMOIRE_STORAGE", &root);
        assert_eq!(store_init(g.to_str().unwrap()), 0);
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create("B", "School", PAPER_BLANK, &mut nb), 0);
        let mut a = pen_record_app();
        a.scr = ScreenId::Library;
        settings_default(&mut a.settings);
        lib(&mut a).modal = Modal::None;
        reload(&mut a);
        assert!(library_shortcut(&mut a, SHORTCUT_FILES));
        assert!(files_shown(&a) && lib(&mut a).files.mode == BrowseMode::Place);
        assert!(lib(&mut a).files.go(crate::fsview::StoragePath::new(b"documents").unwrap()));
        assert!(library_shortcut(&mut a, SHORTCUT_FILES));
        let mut out = String::new();
        let h = a.h;
        assert!(lib(&mut a).files.probe(h, "place", &mut out) && out == "root");
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert!(!files_shown(&a));
        lib(&mut a).cur_folder.set("School");
        reload(&mut a);
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert!(lib(&mut a).cur_folder.is_empty() && lib(&mut a).items[0].is_folder);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        std::env::remove_var("GRIMOIRE_STORAGE");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ---- home (2.2, issue #160) ----

    // A storage with Grimoire's folder in it and a folder of the Kindle's (documents/Planners/2026); three notebooks,
    // A at the Library's top, B in School and C in School/2026; and the app as a start leaves it before the Library
    // first enters.
    fn home_app(name: &str) -> (App, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("grimoire-home-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("documents/Planners/2026")).unwrap();
        std::env::set_var("GRIMOIRE_STORAGE", &root);
        assert_eq!(store_init(root.join("grimoire").to_str().unwrap()), 0);
        for (n, folder) in [("A", ""), ("B", "School"), ("C", "School/2026")] {
            let mut nb = NotebookInfo::default();
            assert_eq!(store_create(n, folder, PAPER_BLANK, &mut nb), 0);
        }
        let mut a = pen_record_app();
        a.scr = ScreenId::Library;
        a.running = true;
        settings_default(&mut a.settings);
        lib(&mut a).modal = Modal::None;
        (a, root)
    }

    fn home_done(mut a: App, root: std::path::PathBuf) {
        library_free(&mut a);
        canvas_free(&mut a.screen);
        std::env::remove_var("GRIMOIRE_STORAGE");
        let _ = std::fs::remove_dir_all(&root);
    }

    // The Library as it comes in front at a start with settings.ini's home= as `value` (read as the file's line is).
    fn home_start(a: &mut App, value: &str) {
        let s = lib(a);
        s.files.close();
        s.cur_folder.clear();
        s.modal = Modal::None;
        s.home_pending = true;
        a.toast_visible = false;
        a.settings.home.clear();
        a.settings.home_refused = false;
        match Home::parse(value.as_bytes()) {
            Some(h) => {
                a.settings.home.set(&h.value());
            }
            None => a.settings.home_refused = true,
        }
        settings_save(&a.settings);
        library_enter(a);
    }

    fn files_place_now(a: &mut App) -> String {
        let (mut out, h) = (String::new(), a.h);
        assert!(lib(a).files.probe(h, "place", &mut out));
        out
    }

    fn home_line(root: &std::path::Path) -> Option<String> {
        let text = std::fs::read_to_string(root.join("grimoire/settings.ini")).unwrap_or_default();
        text.lines().find_map(|l| l.strip_prefix("home=")).map(|v| v.to_string())
    }

    // A folder's menu opened from its card's `...`, and its home row (Set as home, or Reset home to Notebooks).
    fn home_menu_row(a: &mut App, folder: &str) -> Hit {
        let i = lib(a).items.iter().position(|it| it.is_folder && it.folder.as_str() == folder).unwrap_or_else(|| panic!("no folder {folder}"));
        act(a, &Hit { id: H_CARD_MORE, arg: i as i32, ..Default::default() });
        assert_eq!(lib(a).modal, Modal::FolderMore);
        lib(a).hits.h.iter().find(|h| h.id == H_FMORE_HOME).copied().expect("the home row")
    }

    // Grimoire opens the main menu at home: the Library's top with no home set (nothing changes then, and nothing is
    // written), a Library folder, a folder inside one, or a folder of the Kindle's storage, where Files opens over
    // the Library.
    #[test]
    fn the_main_menu_opens_at_home() {
        let (mut a, root) = home_app("start");
        let _ = std::fs::remove_file(root.join("grimoire/settings.ini"));
        library_enter(&mut a);
        assert!(lib(&mut a).cur_folder.is_empty() && !files_shown(&a) && !a.toast_visible);
        assert_eq!(lib(&mut a).items.len(), 2, "School and A");
        assert!(!root.join("grimoire/settings.ini").exists(), "a start with no home writes no settings");
        home_start(&mut a, "notebooks");
        assert!(lib(&mut a).cur_folder.is_empty() && !files_shown(&a) && !a.toast_visible);
        home_start(&mut a, "notebooks/School");
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School");
        assert_eq!(lib(&mut a).items.len(), 2, "2026 and B");
        assert!(!files_shown(&a) && !a.toast_visible);
        // (the folder that is home keeps its Back button: the Library has no path bar to lead up from it)
        assert!(lib(&mut a).hits.h.iter().any(|h| h.id == H_BACK));
        home_start(&mut a, "notebooks/School/2026");
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School/2026");
        assert!(!a.toast_visible);
        home_start(&mut a, "files/documents/Planners");
        assert!(files_shown(&a) && lib(&mut a).files.mode == BrowseMode::Place && lib(&mut a).files.at_home());
        assert_eq!(files_place_now(&mut a), "documents/Planners");
        assert!(lib(&mut a).cur_folder.is_empty() && !a.toast_visible);
        assert_eq!(home_line(&root).as_deref(), Some("files/documents/Planners"), "home stays as it was set");
        home_done(a, root);
    }

    // A home that is gone, or that Grimoire does not enter, or a line that names no home: the Library's top, "Home
    // folder not found." once, and settings.ini without the line from then on. Nothing is made at the path, and the
    // next time the Library comes in front says nothing.
    #[test]
    fn a_home_that_is_gone_or_refused_is_notebooks_with_one_toast() {
        let (mut a, root) = home_app("gone");
        std::fs::write(root.join("documents/file.pdf"), b"%PDF-1.4\n").unwrap();
        std::os::unix::fs::symlink("Planners", root.join("documents/Linked")).unwrap();
        std::os::unix::fs::symlink("/etc", root.join("documents/etc")).unwrap();
        std::fs::create_dir_all(root.join("system/fonts")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        let outside = root.parent().unwrap().join(format!("grimoire-home-outside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&outside);
        for bad in [
            // gone
            "notebooks/Nowhere",
            "notebooks/School/Nowhere",
            "files/documents/Nowhere",
            "files/documents/Planners/2026/Nowhere",
            // a file where a folder was, a link in the path (to a folder of the storage, and out of it)
            "files/documents/file.pdf",
            "files/documents/Linked",
            "files/documents/Linked/2026",
            "files/documents/etc",
            // places Grimoire never opens at
            "files/grimoire/trash",
            "files/grimoire/notebooks",
            "files/system/fonts",
            "files/.hidden",
            // lines that name no home
            "/etc",
            "files/../etc",
            "files//etc",
            "files/",
            "files/documents/../../outside",
            "somewhere",
        ] {
            home_start(&mut a, bad);
            assert!(lib(&mut a).cur_folder.is_empty() && !files_shown(&a), "{bad}: the Library's top");
            assert_eq!(lib(&mut a).items.len(), 2, "{bad}: School and A");
            assert!(a.toast_visible, "{bad}: said");
            assert_eq!(a.toast.as_str(), "Home folder not found.", "{bad}");
            assert!(a.settings.home.is_empty() && !a.settings.home_refused, "{bad}: home is Notebooks again");
            assert_eq!(home_line(&root), None, "{bad}: settings.ini has no home line");
            // the Library in front again with nothing done in it (a notebook KOReader opened at the start, closed):
            // nothing more is said
            a.toast_visible = false;
            library_enter(&mut a);
            assert!(!a.toast_visible && lib(&mut a).cur_folder.is_empty() && !files_shown(&a), "{bad}: once");
        }
        assert!(!root.join("documents/Nowhere").exists() && !outside.exists(), "nothing is made at a home that fails");
        // the file's own line, read as a start reads it: a hostile value is not kept, and it is said once
        std::fs::write(root.join("grimoire/settings.ini"), b"mirror_pdf=1\nhome=/mnt/us/documents\n").unwrap();
        settings_load(&mut a.settings);
        assert!(a.settings.home.is_empty() && a.settings.home_refused);
        lib(&mut a).home_pending = true;
        library_enter(&mut a);
        assert!(a.toast_visible && a.toast.as_str() == "Home folder not found." && !a.settings.home_refused);
        assert_eq!(home_line(&root), None);
        home_done(a, root);
    }

    // Home is where the main menu opens, not where it comes back to: once a contact, a gesture or an action has
    // reached it, the Library comes in front where it was left (a notebook closed). Until then it opens at home
    // again (a notebook KOReader started Grimoire on, closed).
    #[test]
    fn home_is_where_the_menu_opens_until_it_is_used() {
        let (mut a, root) = home_app("pending");
        home_start(&mut a, "notebooks/School");
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School");
        // nothing done: in front again, at home again, wherever it was put meanwhile
        lib(&mut a).cur_folder.clear();
        library_enter(&mut a);
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School");
        // a tap on its Back (a contact): the top, and that is where a notebook's closing comes back to
        let back = lib(&mut a).hits.h.iter().find(|h| h.id == H_BACK).copied().expect("Back, in a folder");
        let (x, y) = ((back.r.x + back.r.w / 2) as f32, (back.r.y + back.r.h / 2) as f32);
        for type_ in [InType::TouchDown, InType::TouchUp] {
            library_event(&mut a, &InEvent { type_, x, y, ..Default::default() });
        }
        assert!(lib(&mut a).cur_folder.is_empty() && !lib(&mut a).home_pending);
        library_enter(&mut a);
        assert!(lib(&mut a).cur_folder.is_empty(), "where it was left, not home");
        // an action counts as a contact does
        lib(&mut a).home_pending = true;
        assert!(!library_shortcut(&mut a, SHORTCUT_UNDO) && !lib(&mut a).home_pending);
        // a pen hovering over it does not
        lib(&mut a).home_pending = true;
        library_event(&mut a, &InEvent { type_: InType::PenHover, x: 500.0, y: 900.0, ..Default::default() });
        assert!(lib(&mut a).home_pending);
        home_done(a, root);
    }

    // The `home` action goes to the home folder from the Library and from Files. At a home inside Files Esc asks
    // "Exit Grimoire?" as at the Library's top (a folder inside home goes up to it first), over Files, where Enter and
    // Esc answer it as they do over the Library; at a Library folder that is home Esc goes up, as its Back button
    // does. A home that is gone when the action goes there is Notebooks, said once.
    #[test]
    fn the_home_action_goes_home_and_esc_asks_there() {
        let (mut a, root) = home_app("action");
        reload(&mut a);
        // no home: the top, as before
        lib(&mut a).cur_folder.set("School");
        reload(&mut a);
        assert!(library_shortcut(&mut a, SHORTCUT_HOME) && lib(&mut a).cur_folder.is_empty());
        assert!(library_back(&mut a) && lib(&mut a).modal == Modal::Exit);
        close_modal(&mut a);
        // a Library folder
        home_save(&mut a, Home::Library("School".into()));
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School");
        assert!(library_back(&mut a) && lib(&mut a).modal == Modal::None, "Esc at a Library home goes up, as Back does");
        assert!(lib(&mut a).cur_folder.is_empty());
        lib(&mut a).cur_folder.set("School/2026");
        reload(&mut a);
        assert!(library_back(&mut a) && lib(&mut a).modal == Modal::None, "inside home: up");
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School");
        // (its Back button still leads up, and the top asks as it always did)
        redraw(&mut a, RefreshMode::Ui);
        act(&mut a, &Hit { id: H_BACK, ..Default::default() });
        assert!(lib(&mut a).cur_folder.is_empty());
        assert!(library_back(&mut a) && lib(&mut a).modal == Modal::Exit);
        close_modal(&mut a);
        // from Files, home in the Library: out of Files
        assert!(library_shortcut(&mut a, SHORTCUT_FILES) && files_shown(&a));
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert!(!files_shown(&a) && lib(&mut a).cur_folder.as_str() == "School");
        // a folder inside Files: from the Library, and from elsewhere in Files
        home_save(&mut a, Home::Files(StoragePath::new(b"documents/Planners").unwrap()));
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert!(files_shown(&a) && files_place_now(&mut a) == "documents/Planners");
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School", "the Library under Files stays where it was");
        assert!(library_shortcut(&mut a, SHORTCUT_FILES) && files_place_now(&mut a) == "root");
        assert!(library_shortcut(&mut a, SHORTCUT_HOME) && files_place_now(&mut a) == "documents/Planners");
        // Esc there asks, over Files; Esc again is its No, and Files is as it was
        assert!(library_shortcut(&mut a, SHORTCUT_BACK));
        assert!(lib(&mut a).modal == Modal::Exit && files_shown(&a) && !files_in_front(&a));
        assert!(lib(&mut a).hits.h.iter().any(|h| h.id == H_EXIT_YES), "its answers are the targets");
        // (Files has work left that draws: the home folder was read again, and its rows' stats come on its ticks. Under
        // the question none of it runs: nothing is drawn or asked to be refreshed, and the answers stay the targets)
        lib(&mut a).files.relist(now_ms());
        let changes = a.changes;
        for _ in 0..5 {
            library_tick(&mut a, now_ms());
        }
        assert_eq!(a.changes, changes, "Files does not draw over it");
        assert!(lib(&mut a).hits.h.iter().any(|h| h.id == H_EXIT_YES));
        assert!(library_modal_key(&mut a, 1));
        assert!(lib(&mut a).modal == Modal::None && files_shown(&a) && files_place_now(&mut a) == "documents/Planners");
        // Enter is its Yes
        assert!(library_back(&mut a) && lib(&mut a).modal == Modal::Exit);
        assert!(a.running && library_modal_key(&mut a, 28) && !a.running);
        a.running = true;
        // a folder inside home: Esc goes up to home, no further
        assert!(lib(&mut a).files.go(StoragePath::new(b"documents/Planners/2026").unwrap()));
        assert!(library_back(&mut a) && lib(&mut a).modal == Modal::None);
        assert_eq!(files_place_now(&mut a), "documents/Planners");
        // gone when the action goes there: Notebooks, said, and home is Notebooks again
        std::fs::remove_dir_all(root.join("documents/Planners")).unwrap();
        assert!(library_shortcut(&mut a, SHORTCUT_FILES) && files_place_now(&mut a) == "root");
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert!(!files_shown(&a) && lib(&mut a).cur_folder.is_empty());
        assert!(a.toast_visible && a.toast.as_str() == "Home folder not found.");
        assert!(a.settings.home.is_empty() && home_line(&root).is_none());
        home_done(a, root);
    }

    // A folder's menu in the Library sets it as home, and on the folder that is home puts home back to Notebooks
    // (said by the row itself the next time the menu opens: no toast). A home folder removed is found gone the next
    // time Grimoire goes home.
    #[test]
    fn a_library_folders_menu_sets_and_resets_home() {
        let (mut a, root) = home_app("folder");
        reload(&mut a);
        let h = home_menu_row(&mut a, "School");
        assert_eq!(h.arg, 0, "Set as home");
        act(&mut a, &h);
        assert!(lib(&mut a).modal == Modal::None && !a.toast_visible);
        assert_eq!(a.settings.home.as_str(), "notebooks/School");
        assert_eq!(home_line(&root).as_deref(), Some("notebooks/School"));
        assert_eq!(home_menu_row(&mut a, "School").arg, 1, "Reset home to Notebooks, on the folder that is home");
        close_modal(&mut a);
        lib(&mut a).cur_folder.set("School");
        reload(&mut a);
        assert_eq!(home_menu_row(&mut a, "School/2026").arg, 0, "a folder inside it can be home");
        close_modal(&mut a);
        // home goes there
        lib(&mut a).cur_folder.clear();
        reload(&mut a);
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert_eq!(lib(&mut a).cur_folder.as_str(), "School");
        assert!(!a.toast_visible);
        // Reset home to Notebooks, on the folder that is home
        lib(&mut a).cur_folder.clear();
        reload(&mut a);
        let h = home_menu_row(&mut a, "School");
        assert_eq!(h.arg, 1);
        act(&mut a, &h);
        assert!(a.settings.home.is_empty() && home_line(&root).is_none());
        assert_eq!(home_menu_row(&mut a, "School").arg, 0, "Set as home again");
        close_modal(&mut a);
        // a home folder removed (an empty one) is found gone when Grimoire goes home
        assert_eq!(store_add_folder("Empty"), 0);
        home_save(&mut a, Home::Library("Empty".into()));
        assert_eq!(store_remove_folder("Empty"), 0);
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert!(lib(&mut a).cur_folder.is_empty() && a.toast_visible && a.toast.as_str() == "Home folder not found.");
        assert!(a.settings.home.is_empty() && home_line(&root).is_none());
        home_done(a, root);
    }

    // Files' own rows: Set as home on the folder shown or a folder's card is kept only after the walk has reached the
    // folder again; Reset home to Notebooks leaves Files where it is; the Go to sheet's Home row goes home.
    #[test]
    fn files_sets_and_resets_home_through_the_library() {
        let (mut a, root) = home_app("filesrows");
        reload(&mut a);
        assert!(library_shortcut(&mut a, SHORTCUT_FILES) && files_shown(&a));
        let planners = StoragePath::new(b"documents/Planners").unwrap();
        files_result(&mut a, BrowserResult::SetHome(planners.clone()));
        assert_eq!(a.settings.home.as_str(), "files/documents/Planners");
        assert_eq!(home_line(&root).as_deref(), Some("files/documents/Planners"));
        assert!(!a.toast_visible && files_shown(&a));
        // a folder that is no longer one to open at is not kept, and it is said as a folder that does not open
        for bad in ["documents/Nowhere", "grimoire/trash", ""] {
            files_result(&mut a, BrowserResult::SetHome(StoragePath::new(bad.as_bytes()).unwrap()));
            assert_eq!(a.settings.home.as_str(), "files/documents/Planners", "{bad}");
            assert!(a.toast_visible && a.toast.as_str() == "Couldn't open folder.", "{bad}");
            a.toast_visible = false;
        }
        files_result(&mut a, BrowserResult::GoHome);
        assert!(files_shown(&a) && files_place_now(&mut a) == "documents/Planners" && lib(&mut a).files.at_home());
        files_result(&mut a, BrowserResult::ResetHome);
        assert!(a.settings.home.is_empty() && home_line(&root).is_none());
        assert!(files_shown(&a) && files_place_now(&mut a) == "documents/Planners" && !lib(&mut a).files.at_home());
        // home is Notebooks: the Home row leaves Files for the Library's top
        lib(&mut a).cur_folder.set("School");
        files_result(&mut a, BrowserResult::GoHome);
        assert!(!files_shown(&a) && lib(&mut a).cur_folder.is_empty() && !a.toast_visible);
        home_done(a, root);
    }

    // A home inside Files waits as Grimoire starts: for What's new (the first start after an update), which shows
    // over the Library and whose closing goes home; and once for a notebook KOReader started Grimoire on, which
    // opens over the Library's cards as before, home opening when the main menu next comes in front.
    #[test]
    fn a_home_in_files_waits_for_whats_new_and_for_koreaders_notebook() {
        let (mut a, root) = home_app("waits");
        a.settings.home.set("files/documents/Planners");
        a.settings.whatsnew_due = true;
        library_enter(&mut a);
        assert!(lib(&mut a).modal == Modal::About && !files_shown(&a) && lib(&mut a).home_deferred);
        // (its Dismiss is a contact on the main menu)
        lib(&mut a).home_pending = false;
        close_modal(&mut a);
        assert!(lib(&mut a).modal == Modal::None && files_shown(&a) && files_place_now(&mut a) == "documents/Planners");
        assert!(!lib(&mut a).home_deferred);
        // a Library folder that is home shows under What's new at once
        lib(&mut a).files.close();
        a.settings.home.set("notebooks/School");
        a.settings.whatsnew_due = true;
        lib(&mut a).home_pending = true;
        library_enter(&mut a);
        assert!(lib(&mut a).modal == Modal::About && lib(&mut a).cur_folder.as_str() == "School" && !lib(&mut a).home_deferred);
        close_modal(&mut a);
        assert!(!files_shown(&a));
        // KOReader started Grimoire on a book: the Library's cards stay in front for its notebook, once
        a.settings.home.set("files/documents/Planners");
        lib(&mut a).cur_folder.clear();
        lib(&mut a).home_pending = true;
        std::env::set_var("GRIMOIRE_HOST", "koreader");
        std::env::set_var("GRIMOIRE_BOOK", "1");
        library_enter(&mut a);
        assert!(!files_shown(&a) && library_cards_in_front(&a) && lib(&mut a).home_pending);
        library_enter(&mut a);
        assert!(files_shown(&a) && files_place_now(&mut a) == "documents/Planners", "the main menu next in front: home");
        // the same start under What's new: its one wait is taken there, so home opens after that notebook
        lib(&mut a).files.close();
        lib(&mut a).home_held = false;
        lib(&mut a).home_pending = true;
        a.settings.whatsnew_due = true;
        library_enter(&mut a);
        assert!(lib(&mut a).modal == Modal::About && !files_shown(&a) && lib(&mut a).home_held);
        library_enter(&mut a);
        assert!(files_shown(&a) && files_place_now(&mut a) == "documents/Planners", "after the notebook: home");
        std::env::remove_var("GRIMOIRE_HOST");
        std::env::remove_var("GRIMOIRE_BOOK");
        home_done(a, root);
    }

    // A rename, a move or a merge of the Library folder that is home, or of a folder it is in, takes home along, so
    // Grimoire opens there as before; a folder whose name only begins the same is another folder.
    #[test]
    fn a_rename_move_or_merge_of_the_library_folder_takes_home_along() {
        let (mut a, root) = home_app("folderrename");
        reload(&mut a);
        // (the Rename folder keyboard's Done)
        let rename = |a: &mut App, folder: &str, to: &str| {
            let s = lib(a);
            s.target = s.items.iter().position(|it| it.is_folder && it.folder.as_str() == folder).unwrap() as i32;
            s.kbp = KbPurpose::RenameFolder;
            s.kb_parent = Modal::None;
            keyboard_open(&mut s.kb, "", to.as_bytes());
            s.modal = Modal::Kb;
            kb_finished(a, true);
        };
        home_save(&mut a, Home::Library("School".into()));
        rename(&mut a, "School", "Uni");
        assert_eq!(lib(&mut a).folders, ["Uni", "Uni/2026"]);
        assert_eq!(a.settings.home.as_str(), "notebooks/Uni");
        assert_eq!(home_line(&root).as_deref(), Some("notebooks/Uni"));
        // a folder home is in, renamed; and one whose name only begins the same, which is not it
        home_save(&mut a, Home::Library("Uni/2026".into()));
        assert_eq!(store_add_folder("Uni2"), 0);
        reload(&mut a);
        rename(&mut a, "Uni2", "Other");
        assert_eq!(a.settings.home.as_str(), "notebooks/Uni/2026");
        rename(&mut a, "Uni", "School");
        assert_eq!(a.settings.home.as_str(), "notebooks/School/2026");
        // moved into another folder
        let it = *lib(&mut a).items.iter().find(|it| it.is_folder && it.folder.as_str() == "School").unwrap();
        assert!(!move_folder(&mut a, &it, "Other"));
        reload(&mut a);
        assert_eq!(lib(&mut a).folders, ["Other", "Other/School", "Other/School/2026"]);
        assert_eq!(a.settings.home.as_str(), "notebooks/Other/School/2026");
        // merged into a folder that is there, and kept beside one
        assert_eq!(store_add_folder("School"), 0);
        assert!(merge_asked(&mut a, "Other/School", "School"));
        merge_answer(&mut a, false);
        assert_eq!(a.settings.home.as_str(), "notebooks/School/2026");
        assert_eq!(store_add_folder("Other/School"), 0);
        assert!(merge_asked(&mut a, "School", "Other/School"));
        merge_answer(&mut a, true);
        assert_eq!(a.settings.home.as_str(), "notebooks/Other/School 2/2026");
        assert_eq!(home_line(&root).as_deref(), Some("notebooks/Other/School 2/2026"));
        // home goes there
        assert!(library_shortcut(&mut a, SHORTCUT_HOME));
        assert_eq!(lib(&mut a).cur_folder.as_str(), "Other/School 2/2026");
        assert!(!a.toast_visible);
        home_done(a, root);
    }

    // A node whose notebook does not open (its notebook.ini gone since the index read it): the Graph again,
    // and the editor's "Couldn't open notebook" stays up (graph_open used to clear it at once).
    #[test]
    fn a_node_that_does_not_open_keeps_its_toast_over_the_graph() {
        let root = std::env::temp_dir().join(format!("grimoire-graph-toast-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let (mut x, mut y) = (NotebookInfo::default(), NotebookInfo::default());
        assert_eq!(store_create("X", "", PAPER_BLANK, &mut x), 0);
        assert_eq!(store_create("Y", "", PAPER_BLANK, &mut y), 0);
        let mut nav = Nav::default();
        nav_init(&mut nav);
        nav_load(x.id.as_str(), &mut nav);
        let target = format!("n:{}:p{}", y.id.as_str(), y.page_ids[0]);
        assert_eq!(nav_link_set(&mut nav, x.page_ids[0], 1, &target), 0);
        assert_eq!(nav_save_links(x.id.as_str(), &mut nav, &x), 0);
        let mut n = 0;
        while libindex_step() {
            n += 1;
            assert!(n < 10_000);
        }
        let mut a = pen_record_app();
        editor_init(&mut a);
        lib(&mut a).modal = Modal::None;
        a.scr = ScreenId::Library;
        assert!(graph_open(&mut a, None));
        std::fs::remove_file(root.join("notebooks").join(y.id.as_str()).join("notebook.ini")).unwrap();
        graph_result(&mut a, MapResult::Target(FixedStr::from_str(&target)));
        assert_eq!(a.scr, ScreenId::Library);
        assert!(graph_shown(&a), "the Graph again");
        assert!(a.toast_visible);
        assert_eq!(a.toast.as_str(), "Couldn't open notebook");
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A library past the Graph of all notebooks' caps (here 10000 linked pages): the Graph from the Library says
    // so and opens the notebooks' browser, a notebook picked there is its Graph, and Back from the browser alone
    // is the Library (before, the Graph closed at once and showed nothing).
    #[test]
    fn past_the_caps_the_librarys_graph_opens_the_browser() {
        let root = std::env::temp_dir().join(format!("grimoire-graph-caps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        mapview_forget_all_ini();
        for k in 0..5 {
            let dir = root.join(format!("notebooks/cap{k}"));
            std::fs::create_dir_all(&dir).unwrap();
            let pages = (1..=2000).map(|p| p.to_string()).collect::<Vec<_>>().join(",");
            std::fs::write(dir.join("notebook.ini"), format!("name=Cap {k}\nfolder=\npaper=4\ncreated=1\nmodified=1\nlast_page=0\nnext_page_id=2001\npage_w=1860\npage_h=2480\npages={pages}\n")).unwrap();
            let links: String = (1..2000).map(|p| format!("link.{p}=00000001,p{}\n", p + 1)).collect();
            std::fs::write(dir.join("links.ini"), links).unwrap();
        }
        let mut n = 0;
        while libindex_step() {
            n += 1;
            assert!(n < 10_000);
        }
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::None;
        a.scr = ScreenId::Library;
        assert!(graph_open(&mut a, None), "the browser opens");
        assert_eq!(a.toast.as_str(), "Too many linked pages to draw all notebooks.");
        assert!(lib(&mut a).map_picker.open && !lib(&mut a).map.open);
        assert!(graph_shown(&a) && !library_touch_view(&a).zoom_ok);
        // A notebook picked: its own Graph.
        graph_pick(&mut a, "cap1");
        assert!(lib(&mut a).map.open && !lib(&mut a).map_picker.open);
        assert_eq!(mapview_pick(&lib(&mut a).map).map(|p| p.as_str().to_string()).as_deref(), Some("cap1"));
        // Back from the Graph is the Library; open again and Back from the browser alone is the Library too.
        assert!(library_shortcut(&mut a, SHORTCUT_GO_BACK));
        assert!(!graph_shown(&a));
        assert!(graph_open(&mut a, None));
        assert!(library_shortcut(&mut a, SHORTCUT_GO_BACK));
        assert!(!graph_shown(&a));
        while bg_busy() {
            let mut k = -1;
            bg_poll_job(Some(&mut k), None);
            sleep_ms(2);
        }
        mapview_forget_all_ini();
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Two fingers spread over the Library's Graph zoom it, as over the editor's (touchgest needs zoom_ok, or it
    // calls the spread a swipe): with Pinch zoom on, never over the Library's cards or the notebooks'
    // browser.
    #[test]
    fn a_spread_over_the_librarys_graph_is_a_zoom() {
        let mut a = App { scr: ScreenId::Library, ..App::default() };
        a.settings.pinch_zoom = true;
        a.lib = Some(Box::default());
        assert!(!library_touch_view(&a).zoom_ok, "the cards do not zoom");
        lib(&mut a).map.open = true;
        assert!(library_touch_view(&a).zoom_ok, "the Graph zooms");
        lib(&mut a).map_picker.open = true;
        assert!(!library_touch_view(&a).zoom_ok, "the browser over it does not");
        lib(&mut a).map_picker.open = false;
        a.settings.pinch_zoom = false;
        assert!(!library_touch_view(&a).zoom_ok, "Pinch zoom off");
        a.lib = None;
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
        // Clear page has no gesture built in (2.1.0-beta.2); more pen slots are still one source.
        gesture_binding_text_for_caps(&a, SHORTCUT_CLEAR_PAGE, 2, &mut text);
        assert_eq!(text, "no gesture");
        let triple = GestureKey::Pen { button: PenBtn::Button, kind: PenKind::Triple };
        assert_eq!(gesturemap_set(&mut a.gestures, triple, SHORTCUT_CLEAR_PAGE), 0);
        gesture_binding_text_for_caps(&a, SHORTCUT_CLEAR_PAGE, 2, &mut text);
        assert_eq!(text, "pen button triple-click");
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
        assert_eq!(a.lib.as_ref().unwrap().hits.h.len(), 4);
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

        // Disable saves "nothing" for the action and its pair, built-in defaults included (to a root of
        // its own: the test's App has none, and "/gestures.ini" cannot be written).
        let root = std::env::temp_dir().join(format!("grimoire-record-disable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        lib(&mut a).gestures_file_readable = true;
        let group = (0..ACTION_GROUPS.len()).find(|&g| binding_row_index(GESTURES, g, SHORTCUT_NEXT_PAGE).is_some()).unwrap();
        lib(&mut a).gestures_group = group as i32;
        open_record_card(&mut a, SHORTCUT_NEXT_PAGE);
        lib(&mut a).record_next_action = SHORTCUT_PREV_PAGE;
        act(&mut a, &Hit { id: H_RECORD_DISABLE, ..Default::default() });
        assert!(!library_record_active(&a));
        // (the toast names the pair as its row does, both directions being gone)
        let pair = binding_row_at(GESTURES, group, binding_row_index(GESTURES, group, SHORTCUT_NEXT_PAGE).unwrap()).unwrap();
        assert_eq!(a.toast.as_str(), cformat!("No gestures for %s now.", pair.label.unwrap()));
        assert_eq!(gesturemap_action(&a.gestures, GestureKey::Touch { fingers: 2, kind: TouchKind::Left }), 0);
        assert_eq!(gesturemap_action(&a.gestures, GestureKey::Touch { fingers: 2, kind: TouchKind::Right }), 0);
        let mut written = Vec::new();
        assert!(gesturemap_format(&a.gestures, &mut written));
        let written = String::from_utf8_lossy(&written);
        assert!(written.contains("touch.2.left=nothing"));
        assert!(written.contains("touch.2.right=nothing"));
        // (and the file says so)
        let file = std::fs::read_to_string(gesture_path()).unwrap();
        assert!(file.contains("touch.2.left=nothing") && file.contains("touch.2.right=nothing"));
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
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
        assert_eq!(group_row_count(0), 11); // (Select, 2.0.0-beta.2)
        // the nine pens, Previous / next tool and in the picker, Smaller / larger size (2.2: the size keys)
        assert_eq!(group_row_count(1), TOOL_PENS as usize + 3);
        assert_eq!(group_row_count(1), GROUP_ROWS_MAX);
        let size = learn_row_at(1, TOOL_PENS as usize + 2).unwrap();
        assert_eq!((size.label, size.act), (Some("Smaller / larger size"), [SHORTCUT_SIZE_SMALLER, SHORTCUT_SIZE_LARGER]));
        assert_eq!((binding_row_count(BUTTONS, 1), binding_row_count(GESTURES, 1)), (12, 12));
        assert_eq!(binding_row_index(BUTTONS, 1, SHORTCUT_SIZE_LARGER), Some(11));
        assert_eq!(binding_row_index(GESTURES, 1, SHORTCUT_SIZE_SMALLER), Some(11));
        // (quick-size has no row: it is bound by name)
        assert!((0..N_GROUPS).all(|g| binding_row_index(BUTTONS, g, SHORTCUT_QUICK_SIZE).is_none()));
        // Zoom and scrolling are Pages rows; Index owns links and naming.
        assert_eq!(group_row_count(2), 11);
        assert_eq!(binding_row_count(BUTTONS, 2), 9);
        assert_eq!(binding_row_count(GESTURES, 2), 11);
        assert_eq!(learn_row_at(2, 2).unwrap().act, [SHORTCUT_ZOOM_OUT, SHORTCUT_ZOOM_IN]);
        assert_eq!(learn_row_at(2, 7).unwrap().act, [SHORTCUT_LANDSCAPE_FIT, 0]);
        assert_eq!(learn_row_at(2, 8).unwrap().act, [SHORTCUT_GESTURE_NEW_PAGE, 0]);
        assert_eq!(group_row_count(3), 7);
        assert_eq!(ACTION_GROUPS[3].probe, "index");
        assert_eq!(learn_row_at(3, 6).unwrap().act, [SHORTCUT_LINK_AREAS, 0]);
        assert_eq!(group_row_count(4), 9);
        assert!(learn_row_at(4, 9).is_none());
        assert_eq!(group_row_count(5), 10);
        assert_eq!(group_row_count(6), 6);
        assert_eq!(binding_row_count(BUTTONS, 6), 4);
        assert_eq!(binding_row_count(GESTURES, 0), 11); // Hold eraser is pen-only here; Select is there.
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

    /// The size keys' row (2.2), last in Tools on both pages, upright and sideways: Gestures names it with its
    /// gesture ("Smaller / larger size: no gesture"), and Buttons has a twelfth row to tap, which learns the smaller
    /// size's button first, as every paired row does.
    #[test]
    fn the_size_keys_row_is_last_in_tools_on_both_pages() {
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        for (w, h) in [(1860, 2480), (2480, 1860)] {
            let mut a = App { w, h, dpi: 300.0, ..Default::default() };
            assert_eq!(canvas_init(&mut a.screen, w, h), 0);
            library_init(&mut a);
            gesturemap_defaults(&mut a.gestures);
            lib(&mut a).modal = Modal::Gestures;
            lib(&mut a).gestures_group = GROUP_TOOLS as i32;
            hits_clear(&mut lib(&mut a).hits);
            draw_modal(&mut a);
            assert_eq!(probe(&mut a, "gestures_group"), "tools");
            assert_eq!(probe(&mut a, "gestures.12"), "Smaller / larger size: no gesture");
            assert_eq!(probe(&mut a, "ui_has.gestures.row.12"), "1");
            assert_eq!(probe(&mut a, "ui_has.gestures.row.13"), "0");
            lib(&mut a).modal = Modal::Buttons;
            bind_group_set(BUTTONS, lib(&mut a), GROUP_TOOLS as i32);
            hits_clear(&mut lib(&mut a).hits);
            draw_modal(&mut a);
            assert_eq!(probe(&mut a, "buttons_group"), "tools");
            assert_eq!(probe(&mut a, "ui_has.buttons.row.12"), "1");
            assert_eq!(probe(&mut a, "ui_has.buttons.row.13"), "0");
            lib(&mut a).learn_row = 11;
            lib(&mut a).learn_step = 0;
            assert_eq!(probe(&mut a, "learning"), "size-smaller");
            lib(&mut a).learn_step = 1;
            assert_eq!(probe(&mut a, "learning"), "size-larger");
            library_free(&mut a);
            canvas_free(&mut a.screen);
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
                            // (Back at the top left closes the group, as Done used to; the X at the top
                            // right closes all of Settings; then the rows)
                            let done_hit = bind_done_hit(kind);
                            let row_hit = bind_row_hit(kind);
                            assert_eq!(hits.h.len(), binding_row_count(kind, group) + 2);
                            assert_eq!(hits.h.iter().filter(|h| h.id == done_hit).count(), 1);
                            assert_eq!(hits.h.iter().filter(|h| h.id == H_SETTINGS_CLOSE).count(), 1);
                            let mut row_bottom = 0;
                            let mut next_row = 0;
                            for hit in &hits.h {
                                let r = hit.r;
                                assert!(
                                    r.y >= 0 && r.y + r.h < h - ui_px(20.0),
                                    "{:?} group {group}: y {} h {} screen {h}",
                                    kind.modal,
                                    r.y,
                                    r.h
                                );
                                assert!(r.x >= 0 && r.x + r.w <= w);
                                assert!(r.h >= (dpi * 10.0 / 25.4).floor() as i32);
                                let found =
                                    hits_find(hits, (r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32)
                                        .unwrap();
                                assert_eq!((found.id, found.arg), (hit.id, hit.arg));
                                if hit.id == done_hit || hit.id == H_SETTINGS_CLOSE {
                                    assert_eq!(hit.arg, 0);
                                } else {
                                    assert_eq!(hit.id, row_hit);
                                    assert_eq!(hit.arg, next_row);
                                    assert!(r.y > row_bottom, "row {next_row}: y {} bottom {row_bottom}", r.y);
                                    row_bottom = r.y + r.h;
                                    next_row += 1;
                                }
                            }
                            assert_eq!(next_row, binding_row_count(kind, group) as i32);
                        }
                        bind_group_set(kind, lib(&mut a), -1);
                        hits_clear(&mut lib(&mut a).hits);
                        draw_modal(&mut a);
                        for hit in &a.lib.as_ref().unwrap().hits.h {
                            assert!(hit.r.y >= 0 && hit.r.y + hit.r.h < h - ui_px(20.0));
                            assert!(hit.r.x >= 0 && hit.r.x + hit.r.w <= w);
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

    // Settings' page with Wi-Fi: the General category lists it beside Language and Updates, as a whole
    // row in every language and style, upright and sideways.
    #[test]
    fn settings_general_lists_wifi_in_every_language_and_style() {
        let root = std::env::temp_dir().join(format!("grimoire-wifi-row-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        let general = settingsmenu::CATEGORIES
            .iter()
            .position(|c| c.items.iter().any(|it| it.id == settingsmenu::SettingId::Wifi))
            .expect("the Wi-Fi row's category") as i32;
        for lang in ["en", "es", "ja"] {
            for theme in [UiTheme::Gothic, UiTheme::Classic] {
                for (w, h) in [(1860, 2480), (2480, 1860)] {
                    ui_init(300.0);
                    ui_set_theme(theme);
                    i18n_set(lang);
                    let mut a = App { w, h, dpi: 300.0, input_drained: true, ..Default::default() };
                    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
                    library_init(&mut a);
                    lib(&mut a).modal = Modal::Settings;
                    lib(&mut a).settings_cat = general;
                    hits_clear(&mut lib(&mut a).hits);
                    draw_settings(&mut a);
                    let row = a.lib.as_ref().unwrap().hits.h.iter().find(|x| x.id == H_WIFI_MENU).map(|x| x.r)
                        .unwrap_or_else(|| panic!("{lang} {theme:?} {w}: Wi-Fi row missing"));
                    assert!(row.w >= ui_px(320.0), "{lang} {theme:?} {w}: Wi-Fi row {}", row.w);
                    assert!(row.h >= ui_px(90.0), "{lang} {theme:?} {w}: Wi-Fi row {}", row.h);
                    assert!(row.y >= 0 && row.y + row.h <= h, "{lang} {theme:?} {w}: Wi-Fi row off the screen");
                    assert!(!tr("Wi-Fi").is_empty());
                    library_free(&mut a);
                    canvas_free(&mut a.screen);
                }
            }
        }
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Adding a row to settingsmenu.rs is all a row needs: the page draws every visible item with the
    // one shared row design and gives it its hit, in both styles and both orientations. A list
    // longer than the screen is paged, and every page draws only the rows it holds.
    #[test]
    fn every_settings_row_is_drawn_and_tappable() {
        let root = std::env::temp_dir().join(format!("grimoire-settings-rows-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        i18n_set("en");
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            for (w, h) in [(1860, 2480), (2480, 1860)] {
                ui_init(300.0);
                ui_set_theme(theme);
                let mut a = App { w, h, dpi: 300.0, input_drained: true, ..Default::default() };
                assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
                library_init(&mut a);
                lib(&mut a).modal = Modal::Settings;
                for (ci, cat) in settingsmenu::CATEGORIES.iter().enumerate() {
                    let want: Vec<i32> = cat
                        .items
                        .iter()
                        .filter(|it| it.show.visible(theme == UiTheme::Classic, sys_services()))
                        .map(|it| setting_hit(it.id))
                        .collect();
                    let mut got: Vec<i32> = Vec::new();
                    lib(&mut a).settings_cat = ci as i32;
                    lib(&mut a).settings_cat_page = 0;
                    for page in 0..=want.len() {
                        lib(&mut a).settings_rows_page = page as i32;
                        hits_clear(&mut lib(&mut a).hits);
                        draw_settings(&mut a);
                        let s = a.lib.as_ref().unwrap();
                        // (a page past the last is clamped to the last: stop before counting it twice)
                        if s.settings_rows_page != page as i32 {
                            break;
                        }
                        for hit in &s.hits.h {
                            assert!(
                                hit.r.x >= 0 && hit.r.x + hit.r.w <= w && hit.r.y >= 0 && hit.r.y + hit.r.h <= h,
                                "{theme:?} {w}x{h} {}: a hit off the screen",
                                cat.title
                            );
                            if want.contains(&hit.id) {
                                got.push(hit.id);
                            }
                        }
                    }
                    for id in &want {
                        assert!(
                            got.contains(id),
                            "{theme:?} {w}x{h} {}: a row was not drawn or not tappable",
                            cat.title
                        );
                    }
                    assert_eq!(got.len(), want.len(), "{theme:?} {w}x{h} {}: a row drawn twice", cat.title);
                }
                library_free(&mut a);
                canvas_free(&mut a.screen);
            }
        }
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A tap on Settings' white space does nothing at all: the page must not carry a shield hit over
    // all of it, which would be found, invert the whole screen as press feedback, and refresh it
    // whole at the lift (a full-screen flash on every tap between the rows).
    #[test]
    fn a_tap_on_settings_white_space_refreshes_nothing() {
        let root = std::env::temp_dir().join(format!("grimoire-settings-blank-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, input_drained: true, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        library_init(&mut a);
        lib(&mut a).modal = Modal::Settings;
        lib(&mut a).settings_cat = 2;
        hits_clear(&mut lib(&mut a).hits);
        draw_settings(&mut a);
        // Any point no row covers (a gap between the cards, or the title bar).
        let mut blank = None;
        'scan: for y in (0..a.h).step_by(8) {
            for x in (0..a.w).step_by(8) {
                if hits_find(&a.lib.as_ref().unwrap().hits, x as f32, y as f32).is_none() {
                    blank = Some((x as f32, y as f32));
                    break 'scan;
                }
            }
        }
        let (x, y) = blank.expect("Settings' white space (a full-screen hit covers the page)");
        a.dirty = RECT_EMPTY;
        library_event(&mut a, &InEvent { type_: InType::TouchDown, x, y, t_ms: 1000, ..Default::default() });
        assert!(a.lib.as_ref().unwrap().pressed.is_none(), "white space took a press (it would invert the screen)");
        assert!(rect_is_empty(a.dirty), "white space asked for a refresh");
        library_event(&mut a, &InEvent { type_: InType::TouchUp, x, y, t_ms: 1040, ..Default::default() });
        assert!(a.lib.as_ref().unwrap().pressed.is_none());
        assert!(rect_is_empty(a.dirty), "white space refreshed the screen");
        assert_eq!(a.lib.as_ref().unwrap().modal, Modal::Settings);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Settings' Sleep & Clock category: two rows, each a page of its own—the sleep screen and timer,
    // the clock and battery—apart from Display's light and appearance rows.
    #[test]
    fn settings_sleep_and_clock_open_their_own_pages() {
        let root = std::env::temp_dir().join(format!("grimoire-sleep-clock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, input_drained: true, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        library_init(&mut a);
        lib(&mut a).modal = Modal::Settings;
        lib(&mut a).settings_cat = 2;
        hits_clear(&mut lib(&mut a).hits);
        draw_settings(&mut a);
        let hits = &a.lib.as_ref().unwrap().hits;
        assert!(hits.h.iter().any(|h| h.id == H_SET_SLEEP), "the Sleep settings row");
        assert!(hits.h.iter().any(|h| h.id == H_SET_CLOCK), "the Clock settings row");

        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SET_SLEEP, arg: 0 });
        {
            let s = lib(&mut a);
            assert_eq!(s.modal, Modal::Display);
            assert_eq!(s.dm.page, crate::dispmenu::DmPage::Sleep);
        }
        let mut dm = std::mem::take(&mut lib(&mut a).dm);
        dispmenu_draw(&mut dm, &mut a);
        for name in ["sleep", "sleep_after"] {
            assert!(dispmenu_control_point(&dm, name).is_some(), "Sleep settings lost {name}");
        }
        for name in ["size", "corner", "time12", "brightness", "invert", "fit"] {
            assert!(dispmenu_control_point(&dm, name).is_none(), "Sleep settings shows {name}");
        }
        lib(&mut a).dm = dm;

        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SET_CLOCK, arg: 0 });
        {
            let s = lib(&mut a);
            assert_eq!(s.modal, Modal::Display);
            assert_eq!(s.dm.page, crate::dispmenu::DmPage::Clock);
        }
        let mut dm = std::mem::take(&mut lib(&mut a).dm);
        dispmenu_draw(&mut dm, &mut a);
        for name in ["size", "corner", "time12"] {
            assert!(dispmenu_control_point(&dm, name).is_some(), "Clock settings lost {name}");
        }
        for name in ["sleep", "sleep_after", "brightness", "invert", "fit"] {
            assert!(dispmenu_control_point(&dm, name).is_none(), "Clock settings shows {name}");
        }
        lib(&mut a).dm = dm;

        library_free(&mut a);
        canvas_free(&mut a.screen);
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Settings > Pen (2.1): the pen button's choices and the eraser's, each opening its own list as
    // Language does; a pick is saved and comes back to Settings.
    #[test]
    fn pen_settings_rows_open_their_choice_sheets() {
        let root = std::env::temp_dir().join(format!("grimoire-pen-choices-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, input_drained: true, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        library_init(&mut a);
        a.settings.side_button = 0;
        a.settings.eraser_mode = 0;
        lib(&mut a).modal = Modal::Settings;
        lib(&mut a).settings_cat = 3;
        hits_clear(&mut lib(&mut a).hits);
        draw_settings(&mut a);
        for id in [H_SET_PENS, H_SET_SIDE, H_SET_ERASER] {
            assert!(
                a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == id),
                "the Pen row {id}"
            );
        }
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SET_SIDE, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::SideButton);
        let pick = a
            .lib
            .as_ref()
            .unwrap()
            .hits
            .h
            .iter()
            .find(|h| h.id == H_SIDE_PICK && h.arg == 2)
            .map(|h| h.r)
            .expect("the pen's Highlights choice");
        act(&mut a, &Hit { r: pick, id: H_SIDE_PICK, arg: 2 });
        assert_eq!(a.settings.side_button, 2);
        assert_eq!(lib(&mut a).modal, Modal::Settings);

        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SET_ERASER, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::EraserMode);
        assert!(
            a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == H_ERASER_PICK && h.arg == 1),
            "the eraser's Precise choice"
        );
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_ERASER_PICK, arg: 1 });
        assert_eq!(a.settings.eraser_mode, 1);
        assert_eq!(lib(&mut a).modal, Modal::Settings);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Settings > System > About: the device's page with its screen, version and storage bar, Back
    // returning to Settings.
    #[test]
    fn settings_about_shows_the_device_and_its_storage() {
        let root = std::env::temp_dir().join(format!("grimoire-about-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, input_drained: true, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        library_init(&mut a);
        lib(&mut a).modal = Modal::Settings;
        lib(&mut a).settings_cat = 5;
        hits_clear(&mut lib(&mut a).hits);
        draw_settings(&mut a);
        assert!(
            a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == H_SET_DEVICE),
            "the About row"
        );
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SET_DEVICE, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::DeviceInfo);
        assert!(
            a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == H_DEVICE_BACK),
            "About's Back"
        );
        assert!(crate::pdfimport::storage_space().is_some(), "the storage's size and free room");
        let links: Vec<i32> = a.lib.as_ref().unwrap().hits.h.iter().filter(|h| h.id == H_LINK).map(|h| h.arg).collect();
        assert_eq!(links, (0..ABOUT_LINKS.len() as i32).collect::<Vec<_>>(), "About's links");
        assert_eq!(ABOUT_LINKS[1].name, "Grimoire GitHub");
        assert_eq!(ABOUT_LINKS[2].url, "https://github.com/nebuff");
        assert_eq!(ABOUT_LINKS[3].url, "https://github.com/ganten1998");
        // (a person's GitHub link is named after the account it opens: "Ganten's GitHub" for ganten1998)
        assert_eq!(ABOUT_LINKS[3].name, "Ganten's GitHub");
        for link in ABOUT_LINKS {
            let Some(account) = link.url.strip_prefix("https://github.com/").filter(|p| !p.contains('/')) else {
                continue;
            };
            let owner = link.name.split('\'').next().unwrap_or("").to_lowercase();
            assert!(account.starts_with(&owner), "{} opens {}", link.name, link.url);
        }
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_LINK, arg: 0 });
        assert_eq!((lib(&mut a).modal, lib(&mut a).device_link), (Modal::LinkQr, 0));
        assert!(
            a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == H_LINK_BACK),
            "the link's Back"
        );
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_LINK_BACK, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::DeviceInfo);
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_LINK, arg: 2 });
        assert_eq!((lib(&mut a).modal, lib(&mut a).device_link), (Modal::LinkQr, 2));
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_LINK_BACK, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::DeviceInfo);
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_DEVICE_BACK, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::Settings);
        // The X closes all of Settings, from a submenu as from the main page.
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SET_DEVICE, arg: 0 });
        assert!(
            a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == H_SETTINGS_CLOSE),
            "About's X"
        );
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SETTINGS_CLOSE, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::None, "the X closes all of Settings");
        library_free(&mut a);
        canvas_free(&mut a.screen);
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Settings > Updates: one button that carries the version, and a switch for the branch updates come
    // from; opening the page starts the check (with no KPM here, the button says so), and the switch
    // flips and saves.
    #[test]
    fn settings_updates_has_one_button_and_the_branch_switch() {
        let root = std::env::temp_dir().join(format!("grimoire-updates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, input_drained: true, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        library_init(&mut a);
        lib(&mut a).modal = Modal::Settings;
        lib(&mut a).settings_cat = 0;
        hits_clear(&mut lib(&mut a).hits);
        draw_settings(&mut a);
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_UPDATES, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::Updates);
        let hits = &a.lib.as_ref().unwrap().hits;
        assert!(hits.h.iter().any(|h| h.id == H_SETTINGS_BACK), "Updates' Back");
        assert!(hits.h.iter().any(|h| h.id == H_UPD_CHANNEL), "the branch switch");
        assert!(
            !hits.h.iter().any(|h| h.id == H_UPD_NOW),
            "without KPM the Update button is disabled"
        );
        assert_eq!(a.lib.as_ref().unwrap().upd, UPD_IDLE, "the check ran and found no KPM");
        let was = a.settings.updates;
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_UPD_CHANNEL, arg: 0 });
        assert_ne!(a.settings.updates, was, "the switch saves its branch");
        assert_ne!(a.settings.updates, 0, "a picked branch is explicit");
        assert_eq!(lib(&mut a).modal, Modal::Updates);
        assert!(a.lib.as_ref().unwrap().hits.h.iter().any(|h| h.id == H_UPD_CHANNEL));
        act(&mut a, &Hit { r: RECT_EMPTY, id: H_SETTINGS_BACK, arg: 0 });
        assert_eq!(lib(&mut a).modal, Modal::Settings);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        i18n_set("en");
        ui_init(300.0);
        ui_set_theme(UiTheme::Gothic);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Settings > Wi-Fi opened while the Share card's last read still runs (its own read cannot start then):
    // it asks once that one is done, and lists the saved networks even with the radio off.
    #[test]
    fn wifi_sheet_reads_its_networks_after_a_read_still_running() {
        let root = std::env::temp_dir().join(format!("grimoire-wifi-sheet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.join("data").to_str().unwrap()), 0);
        let script = root.join("helper.sh");
        std::fs::write(&script, "case $1 in status) sleep 0.3;; esac\nprintf 'wireless=0\\nenabled=0\\nconnected=0\\n'\n\
            [ \"$1\" = networks ] && printf 'current=\\nsaved=486f6d65\\nsaved=4f6666696365\\nlist=1\\n'\nexit 0\n").unwrap();
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Settings;
        wifi_init(script.to_str());
        assert!(wifi_request(WifiAction::Status));
        wifi_sheet_open(&mut a);
        assert_eq!(lib(&mut a).modal, Modal::Wifi);
        assert!(!lib(&mut a).wifi.asked);
        let began = std::time::Instant::now();
        while wifi_saved().is_none() && began.elapsed() < std::time::Duration::from_secs(5) {
            wifi_tick();
            wifi_sheet_tick(&mut a, now_ms());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(wifi_saved().map(|v| v.len()), Some(2));
        assert_eq!(wifi_status().map(|st| st.enabled), Some(false));
        wifi_sheet_tick(&mut a, now_ms());
        // Nearby opens first; Saved is the top-right button, and its rows carry a trash button each.
        wifi_sheet_tap(&mut a, H_WIFI_TAB, WIFI_VIEW_SAVED as i32);
        let rows: Vec<_> = a.lib.as_ref().unwrap().hits.h.iter().filter(|h| h.id == H_WIFI_NET).map(|h| h.arg).collect();
        let trash: Vec<_> = a.lib.as_ref().unwrap().hits.h.iter().filter(|h| h.id == H_WIFI_DELETE).map(|h| h.arg).collect();
        assert_eq!(rows, [0, 1]);
        assert_eq!(trash, [0, 1]);
        let hit = a.lib.as_ref().unwrap().hits.h.iter().find(|h| h.id == H_WIFI_DELETE && h.arg == 0).unwrap().r;
        assert_eq!(hits_find(&a.lib.as_ref().unwrap().hits, (hit.x + hit.w / 2) as f32, (hit.y + hit.h / 2) as f32).map(|h| h.id), Some(H_WIFI_DELETE));
        assert_eq!(lib(&mut a).wifi.rows, [b"Home".to_vec(), b"Office".to_vec()]);
        let s = a.lib.as_ref().unwrap();
        assert_eq!(s.hits.h.iter().filter(|h| h.id == H_SETTINGS_BACK).count(), 1, "Wi-Fi: Back");
        assert_eq!(s.hits.h.iter().filter(|h| h.id == H_SETTINGS_CLOSE).count(), 1, "Wi-Fi: X");
        assert!(!s.hits.h.iter().any(|h| h.id == H_CANCEL), "Wi-Fi: the sheet's own Done is gone");
        // Its password keyboard draws over the Wi-Fi page, which stays open under it.
        let ws = &mut lib(&mut a).wifi;
        ws.other = Some(b"Hidden".to_vec());
        ws.view = WIFI_VIEW_OTHER;
        lib(&mut a).modal = Modal::Wifi;
        open_kb_secret(&mut a, KbPurpose::WifiPassword, "Password");
        assert_eq!(a.lib.as_ref().unwrap().kb_parent, Modal::Wifi);
        hits_clear(&mut lib(&mut a).hits);
        draw_modal(&mut a);
        assert!(!rect_is_empty(a.lib.as_ref().unwrap().kb.area), "the keys are over the page");
        assert!(a.lib.as_ref().unwrap().hits.h.is_empty(), "the page under them is not tapped");
        keyboard_close(&mut lib(&mut a).kb);
        lib(&mut a).modal = Modal::Wifi;
        wifi_init(None);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The network list scrolls by dragging (there is no More networks row): every saved network is
    // held in rows, while the list draws only those in its viewport.
    #[test]
    fn the_saved_network_list_scrolls_by_drag() {
        let root = std::env::temp_dir().join(format!("grimoire-wifi-scroll-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.join("data").to_str().unwrap()), 0);
        let script = root.join("helper.sh");
        let mut saved = String::new();
        for i in 0..20 {
            let name = format!("Network {i:02}");
            let hex: String = name.bytes().map(|b| format!("{b:02x}")).collect();
            saved.push_str(&format!("saved={hex}\\n"));
        }
        std::fs::write(
            &script,
            format!(
                "printf 'wireless=0\\nenabled=0\\nconnected=0\\n'\ncase $1 in\n networks) printf 'current=\\n{}list=1\\n';;\nesac\n",
                saved
            ),
        )
        .unwrap();
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Settings;
        wifi_init(script.to_str());
        wifi_sheet_open(&mut a);
        let began = std::time::Instant::now();
        while wifi_saved().is_none() && began.elapsed() < std::time::Duration::from_secs(5) {
            wifi_tick();
            wifi_sheet_tick(&mut a, now_ms());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(wifi_saved().map(|v| v.len()), Some(20));
        wifi_sheet_tap(&mut a, H_WIFI_TAB, WIFI_VIEW_SAVED as i32);
        let (list, max) = {
            let ws = &lib(&mut a).wifi;
            (ws.list, ws.scroll_max)
        };
        assert!(max > 0, "the twenty saved networks overflow the viewport");
        assert_eq!(lib(&mut a).wifi.rows.len(), 20, "every row is held, not just a page");
        let x = (list.x + list.w / 2) as f32;
        let y0 = (list.y + list.h / 2) as f32;
        let y1 = y0 - ui_px(400.0) as f32;
        library_event(&mut a, &InEvent { type_: InType::TouchDown, x, y: y0, t_ms: 1000, ..Default::default() });
        library_event(&mut a, &InEvent { type_: InType::TouchMove, x, y: y1, t_ms: 1100, ..Default::default() });
        library_event(&mut a, &InEvent { type_: InType::TouchUp, x, y: y1, t_ms: 1200, ..Default::default() });
        assert!(lib(&mut a).wifi.scroll > 0, "a drag up scrolls the list");
        assert!(wifi_joining().is_none(), "a dragged row is not tapped");
        wifi_init(None);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_beta_badge_is_only_for_enterprise_eap() {
        assert!(wifi_beta(Some(WifiSecurity::Enterprise)));
        for sec in [
            WifiSecurity::Open,
            WifiSecurity::Wep,
            WifiSecurity::Wpa,
            WifiSecurity::Wpa2,
            WifiSecurity::Wpa3,
            WifiSecurity::Owe,
            WifiSecurity::Enterprise192,
            WifiSecurity::Unknown,
        ] {
            assert!(!wifi_beta(Some(sec)), "{sec:?}");
        }
        assert!(!wifi_beta(None));
    }

    #[test]
    fn the_enterprise_flow_asks_for_a_username_then_a_masked_password() {
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Wifi;
        wifi_eap_start(
            &mut a,
            WifiNetwork {
                essid: b"School".to_vec(),
                security: WifiSecurity::Enterprise,
                signal: 0,
                signal_max: 1,
                known: false,
            },
        );
        assert_eq!(a.lib.as_ref().unwrap().modal, Modal::Kb);
        assert_eq!(a.lib.as_ref().unwrap().kbp, KbPurpose::WifiEapUser);
        lib(&mut a).kb.text.set("student@example.edu");
        kb_finished(&mut a, true);
        assert_eq!(a.lib.as_ref().unwrap().kbp, KbPurpose::WifiEapPassword);
        assert!(a.lib.as_ref().unwrap().kb.secret);
        assert_eq!(a.lib.as_ref().unwrap().wifi.eap_user.as_deref(), Some(b"student@example.edu".as_slice()));
        kb_finished(&mut a, false);
        assert_eq!(a.lib.as_ref().unwrap().modal, Modal::Wifi);
        assert_eq!(a.lib.as_ref().unwrap().wifi.eap_user, None);
        library_free(&mut a);
        canvas_free(&mut a.screen);
    }

    #[test]
    fn a_scanned_wpa3_network_asks_for_its_password_and_keeps_its_security() {
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Wifi;
        lib(&mut a).wifi.near = vec![WifiNetwork {
            essid: b"Home 3".to_vec(),
            security: WifiSecurity::Wpa3,
            signal: 2,
            signal_max: 5,
            known: false,
        }];
        wifi_sheet_tap(&mut a, H_WIFI_NEAR, 0);
        assert_eq!(a.lib.as_ref().unwrap().kbp, KbPurpose::WifiPassword);
        assert!(a.lib.as_ref().unwrap().kb.secret);
        assert_eq!(
            a.lib.as_ref().unwrap().wifi.add.as_ref().map(|n| n.security),
            Some(WifiSecurity::Wpa3)
        );
        library_free(&mut a);
        canvas_free(&mut a.screen);
    }

    // The nearby page refreshes itself after 20 s of quiet: the scan is asked for without clearing
    // the list, and only its answer changes the rows.
    #[test]
    fn the_nearby_page_asks_for_a_quiet_scan_after_twenty_seconds() {
        let root = std::env::temp_dir().join(format!("grimoire-wifi-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.join("data").to_str().unwrap()), 0);
        let script = root.join("helper.sh");
        let scans = root.join("scans");
        std::fs::write(
            &script,
            format!(
                r#"printf 'wireless=1\nenabled=1\nconnected=1\n'
case $1 in
 scan) echo scan >> '{}'; printf 'scan=ok\nnear=486f6d65,4,5,wpa2,1\nnear_end=1\n';;
 networks) printf 'current=486f6d65\nsaved=486f6d65\nlist=1\n';;
 *) printf 'current=486f6d65\n';;
esac
"#,
                scans.display()
            ),
        )
        .unwrap();
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Settings;
        wifi_init(script.to_str());
        wifi_sheet_open(&mut a);
        let settle = |a: &mut App, scans: &std::path::Path, want: usize| {
            let began = std::time::Instant::now();
            while began.elapsed() < std::time::Duration::from_secs(5) {
                wifi_tick();
                wifi_sheet_tick(a, now_ms());
                let n = std::fs::read_to_string(scans).unwrap_or_default().lines().count();
                if !wifi_busy() && wifi_nearby().is_some() && n >= want {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            panic!("the helper did not finish its scans");
        };
        settle(&mut a, &scans, 1);
        let before = wifi_nearby().unwrap();
        let now = now_ms();
        lib(&mut a).wifi.refresh_at = now.wrapping_sub(WIFI_REFRESH_MS);
        wifi_sheet_tick(&mut a, now);
        assert!(wifi_scan_quieting(), "the idle page asks for a quiet refresh");
        assert_eq!(wifi_nearby(), Some(before), "the list stays while it runs");
        settle(&mut a, &scans, 2);
        assert!(!wifi_scan_quieting());
        assert_eq!(std::fs::read_to_string(&scans).unwrap().lines().count(), 2);
        wifi_init(None);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A scan while a supplicant-owned EAP/WPA3 link is connected can make the Kindle deauth from
    // the AP it just joined. The sheet must leave automatic and quiet scans alone for that link.
    #[test]
    fn a_supplicant_link_keeps_the_nearby_page_from_scanning() {
        let root = std::env::temp_dir().join(format!("grimoire-wifi-supplicant-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.join("data").to_str().unwrap()), 0);
        let script = root.join("helper.sh");
        let scans = root.join("scans");
        std::fs::write(
            &script,
            format!(
                r#"printf 'wireless=1\nenabled=1\nconnected=1\nsupplicant=1\n'
case $1 in
 scan) echo scan >> '{}'; printf 'scan=ok\nnear=486f6d65,4,5,wpa2,1\nnear_end=1\n';;
 networks) printf 'current=486f6d65\nsaved=486f6d65\nlist=1\n';;
 *) printf 'current=486f6d65\n';;
esac
"#,
                scans.display()
            ),
        )
        .unwrap();
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Settings;
        wifi_init(script.to_str());
        wifi_sheet_open(&mut a);
        let began = std::time::Instant::now();
        while began.elapsed() < std::time::Duration::from_secs(1) {
            wifi_tick();
            wifi_sheet_tick(&mut a, now_ms());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(!scans.exists(), "a supplicant-owned link is not scanned");
        assert!(!wifi_scanning());
        wifi_init(None);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The Kindle can report the connected profile as a bracketed second name. The saved list shows
    // it once, plainly, in the Kindle's order, and does not make the connected row special.
    #[test]
    fn saved_networks_are_plain_deduplicated_and_keep_their_order() {
        let root = std::env::temp_dir().join(format!("grimoire-wifi-plain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.join("data").to_str().unwrap()), 0);
        let script = root.join("helper.sh");
        std::fs::write(
            &script,
            r#"case $1 in
 networks) printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=5b486f6d655d\nsaved=4f6666696365\nsaved=5b486f6d655d\nsaved=486f6d65\nlist=1\n';;
 scan) printf 'scan=off\n';;
 *) printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=5b486f6d655d\n';;
esac
"#,
        )
        .unwrap();
        let mut a = pen_record_app();
        lib(&mut a).modal = Modal::Settings;
        wifi_init(script.to_str());
        wifi_sheet_open(&mut a);
        let began = std::time::Instant::now();
        while wifi_saved().is_none() && began.elapsed() < std::time::Duration::from_secs(5) {
            wifi_tick();
            wifi_sheet_tick(&mut a, now_ms());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(wifi_saved().map(|v| v.len()), Some(3), "the raw report still has both names");
        assert!(wifi_is_current(b"Home", Some(b"[Home]")));
        assert!(wifi_is_current(b"[Home]", Some(b"Home")));
        assert!(!wifi_is_current(b"Office", Some(b"[Home]")));
        wifi_sheet_tap(&mut a, H_WIFI_TAB, WIFI_VIEW_SAVED as i32);
        assert_eq!(lib(&mut a).wifi.rows, [b"Office".to_vec(), b"Home".to_vec()]);
        wifi_init(None);
        library_free(&mut a);
        canvas_free(&mut a.screen);
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Resolve a runner tap through the same hit names used by ui_at probes.
pub fn library_control_point(a:&mut App,name:&str)->Option<(f32,f32)> {
    if name.starts_with("music.") {return musicbuild_control_point(&a.lib.as_deref()?.mb,name);}
    if name.starts_with("ui_has.") {return None;}
    // The file picker: files.back, files.row.N, files.more.N, files.item.NAME, files.prev, files.next, files.card.*
    if let Some(n)=name.strip_prefix("files.") {return a.lib.as_deref()?.files.point(n);}
    // The Templates sheet over New notebook (2.2): templates.from_file, templates.back
    if let Some(n)=name.strip_prefix("templates.") {
        let s=a.lib.as_deref()?;
        return (s.modal==Modal::Templates && !s.files.open).then(||tplsheet_point(&s.ts,n)).flatten();
    }
    // The Display sheet's controls while it is open ("display.done", "display.fit").
    if let Some(n)=name.strip_prefix("display.") {return dispmenu_control_point(&a.lib.as_deref()?.dm,n);}
    // The Graph opened from the Library: its controls (map.back, map.view, ...) and its notebooks' browser's
    // past the caps (elsewhere.book.ID, elsewhere.folder.PATH, elsewhere.back), as the editor's panel names them.
    if graph_shown(a) {
        let s=a.lib.as_deref()?;
        if let Some(n)=name.strip_prefix("elsewhere.") {return s.map_picker.open.then(||s.map_picker.point(n)).flatten();}
        if let Some(n)=name.strip_prefix("map.") {return mapview_control_point(&s.map,n);}
        return None;
    }
    // The Files card (2.2) and its menu button.
    if name=="files-card"||name=="files-card-menu" {
        let s=a.lib.as_deref()?;
        let i=s.items.iter().position(|it|it.is_files)?;
        let id=if name=="files-card" {H_CARD} else {H_CARD_MORE};
        return s.hits.h.iter().find(|h|h.id==id&&h.arg==i as i32).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32));
    }
    // A card ("notebook.ID", "folder.PATH"), or its menu button ("notebook-menu.ID", "folder-menu.PATH").
    let card=match name.split_once('.') {
        Some(("notebook",rest))=>Some((false,rest,H_CARD)),
        Some(("folder",rest))=>Some((true,rest,H_CARD)),
        Some(("notebook-menu",rest))=>Some((false,rest,H_CARD_MORE)),
        Some(("folder-menu",rest))=>Some((true,rest,H_CARD_MORE)),
        _=>None,
    };
    if let Some((folder,rest,id))=card {
        let s=a.lib.as_deref()?;
        let i=s.items.iter().position(|it|!it.is_files&&it.is_folder==folder&&if folder{it.folder==rest}else{s.books[it.nb].id==rest})?;
        return s.hits.h.iter().find(|h|h.id==id&&h.arg==i as i32).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32));
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

/// The list's next or previous page (its arrows).
fn turn(a: &mut App, next: bool) {
    let s = lib(a);
    // Only the visible page's thumbnails stay in memory (each is ~0.5 MB).
    for i in 0..s.books.len() {
        canvas_free(&mut s.thumbs[i]);
        s.thumb_tried[i] = false;
    }
    s.page += if next { 1 } else { -1 };
    redraw(a, RefreshMode::Ui);
}

/// The Library's open on-screen keyboard (a name being typed), None for none: where a hardware key's
/// input goes (app_type_text).
pub fn library_keyboard_area(a: &App) -> Option<Rect> {
    let s = a.lib.as_deref()?;
    if s.files.open {
        // (Files' rename keyboard, 2.2: Files takes every contact while it is up)
        return s.files.keyboard_area();
    }
    (s.modal == Modal::Kb && s.kb.open && !rect_is_empty(s.kb.area)).then_some(s.kb.area)
}

/// The notebook hosts a settings dialog and all its nested sheets without changing screens.
pub fn library_settings_active(a:&App)->bool {
    a.scr==ScreenId::Editor && a.lib.as_deref().is_some_and(|s|s.settings_return_editor && !s.settings_painting)
}
pub fn library_settings_redraw(a:&mut App) {redraw(a,RefreshMode::Ui);}
pub fn library_settings_back(a:&mut App) {close_modal(a);}
