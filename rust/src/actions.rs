//! The action registry: stable ids, names, labels, groups, kinds, source masks and learn rows.
//!
//! Action ids are in-memory only. The `name` field is what the INI files keep.

use crate::prelude::*;

pub const SHORTCUT_ZOOM_OUT: i32 = 56;
pub const SHORTCUT_ZOOM_IN: i32 = 57;
pub const SHORTCUT_SCROLL_UP: i32 = 58;
pub const SHORTCUT_SCROLL_DOWN: i32 = 59;
pub const SHORTCUT_INVERT: i32 = 60;
pub const SHORTCUT_LIGHT_DOWN: i32 = 61;
pub const SHORTCUT_LIGHT_UP: i32 = 62;
pub const SHORTCUT_WARMTH_DOWN: i32 = 63;
pub const SHORTCUT_WARMTH_UP: i32 = 64;
pub const SHORTCUT_ROTATION_LOCK: i32 = 65;
pub const SHORTCUT_PINCH_ZOOM: i32 = 66;
pub const SHORTCUT_THEME: i32 = 67;
pub const SHORTCUT_SIDE_BUTTON: i32 = 68;
pub const SHORTCUT_MIRROR_PDF: i32 = 69;
pub const SHORTCUT_SLEEP_SCREEN: i32 = 70;
pub const SHORTCUT_SLEEP: i32 = 71;
pub const SHORTCUT_GESTURES: i32 = 72;
pub const SHORTCUT_SHORTCUTS: i32 = 73;
pub const SHORTCUT_BLUETOOTH_CONTROLLER: i32 = 74;
pub const SHORTCUT_LANGUAGE: i32 = 75;
pub const SHORTCUT_SLEEP_AFTER: i32 = 76;
pub const SHORTCUT_UPDATE_CHANNEL: i32 = 77;
pub const SHORTCUT_SHAPES:i32=78;
pub const SHORTCUT_SHAPE_HOLD:i32=79;
pub const SHORTCUT_SHAPE_LAST:i32=80;
pub const SHORTCUT_INDEX_MAP:i32=81;

pub const SHORTCUT_PEN_PAN: i32 = 82;
pub const SHORTCUT_PEN_ZOOM: i32 = 83;
/// Select mode on or off (the lasso: copy, cut, paste, move; editor/select.rs).
pub const SHORTCUT_SELECT: i32 = 84;
/// Select mode in exact area (the lasso cuts strokes along its line), or with Select mode on, whole
/// strokes or exact area in turn (docs/beta2/PIXEL-LASSO.md 1.10).
pub const SHORTCUT_SELECT_EXACT: i32 = 85;
/// The toolbar's count pad (2.0.0-beta.2): how many blank pages to add after the open page, which stays
/// open (editor.rs add_pad_open; the + held does the same).
pub const SHORTCUT_NEW_PAGES: i32 = 86;
/// The keyboard scheme of 2.0.0-beta.2 (ids 87 to 94): save now (Ctrl+S), back (Esc: one layering for every
/// place), what two fingers up do (Space), and the Library's New notebook, New folder, Share and Settings.
pub const SHORTCUT_SAVE: i32 = 87;
pub const SHORTCUT_BACK: i32 = 88;
pub const SHORTCUT_TWO_FINGER_UP: i32 = 89;
pub const SHORTCUT_NEW_NOTEBOOK: i32 = 90;
pub const SHORTCUT_NEW_FOLDER: i32 = 91;
pub const SHORTCUT_SHARE: i32 = 92;
pub const SHORTCUT_SETTINGS: i32 = 93;
/// Every page ticked in the Pages view (Ctrl+A); the Graph selects every node with it.
pub const SHORTCUT_SELECT_ALL: i32 = 95;
/// The Graph's own keys (ids 100 to 104): its view
/// (View: All and View: Adjacent in turn, fitted) and the arrows' pans (the view moves a sixth of the plot
/// that way). They act only in the Graph (mapview.rs mapview_action).
pub const SHORTCUT_GRAPH_VIEW: i32 = 100;
pub const SHORTCUT_PAN_LEFT: i32 = 101;
pub const SHORTCUT_PAN_RIGHT: i32 = 102;
pub const SHORTCUT_PAN_UP: i32 = 103;
pub const SHORTCUT_PAN_DOWN: i32 = 104;

pub const SRC_KEY: u8 = 1;
pub const SRC_PAD: u8 = 2;
pub const SRC_TOUCH: u8 = 4;
pub const SRC_SHAPE: u8 = 8;
pub const SRC_PEN: u8 = 16;
const SRC_ALL: u8 = SRC_KEY | SRC_PAD | SRC_TOUCH | SRC_SHAPE | SRC_PEN;
const SRC_GESTURES: u8 = SRC_TOUCH | SRC_SHAPE | SRC_PEN;
const SRC_BUTTONS_PEN: u8 = SRC_KEY | SRC_PAD | SRC_PEN;

pub const GROUP_EDITING: usize = 0;
pub const GROUP_TOOLS: usize = 1;
pub const GROUP_PAGES: usize = 2;
pub const GROUP_INDEX: usize = 3;
pub const GROUP_MENUS: usize = 4;
pub const GROUP_SCREEN: usize = 5;
pub const GROUP_SYSTEM: usize = 6;
pub const N_GROUPS: usize = 7;
pub const GROUP_ROWS_MAX: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionKind {
    Toggle,
    Menu,
    Step,
    Cycle,
    OneShot,
    Momentary,
}

#[derive(Clone, Copy, Debug)]
pub struct ActionInfo {
    pub id: i32,
    pub name: &'static str,
    /// The translated label shown for this individual action.
    pub label: &'static str,
    /// gettext context, where the existing UI gives this label a context.
    pub ctx: Option<&'static str>,
    pub icon: IconKind,
    pub group: u8,
    /// The symmetric paired action id, or zero for an unpaired action.
    pub pair: i32,
    pub kind: ActionKind,
    pub sources: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct LearnRow {
    pub label: Option<&'static str>,
    pub act: [i32; 2],
    pub icon: IconKind,
}

#[derive(Clone, Copy, Debug)]
pub struct ActionGroup {
    pub label: &'static str,
    pub title: &'static str,
    pub icon: IconKind,
    pub probe: &'static str,
    pub rows: &'static [LearnRow],
}

const fn ai(
    id: i32,
    name: &'static str,
    label: &'static str,
    ctx: Option<&'static str>,
    icon: IconKind,
    group: usize,
    pair: i32,
    kind: ActionKind,
    sources: u8,
) -> ActionInfo {
    ActionInfo {
        id,
        name,
        label,
        ctx,
        icon,
        group: group as u8,
        pair,
        kind,
        sources,
    }
}

const fn nm(
    id: i32,
    name: &'static str,
    label: &'static str,
    group: usize,
    icon: IconKind,
    kind: ActionKind,
) -> ActionInfo {
    ai(id, name, label, None, icon, group, 0, kind, SRC_ALL)
}
const fn pair(
    id: i32,
    name: &'static str,
    label: &'static str,
    group: usize,
    icon: IconKind,
    pair_id: i32,
    kind: ActionKind,
) -> ActionInfo {
    ai(
        id,
        name,
        label,
        None,
        icon,
        group,
        pair_id,
        kind,
        SRC_ALL,
    )
}

// ---- Buttons and Gestures rows ----

const fn lr(label: Option<&'static str>, act: [i32; 2], icon: IconKind) -> LearnRow {
    LearnRow { label, act, icon }
}

static K_EDITING: [LearnRow; 11] = [
    lr(None, [SHORTCUT_UNDO, 0], IconKind::Undo),
    lr(None, [SHORTCUT_REDO, 0], IconKind::Redo),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_ERASER.0, 0],
        IconKind::Eraser,
    ),
    lr(None, [SHORTCUT_HOLD_ERASER, 0], IconKind::Gamepad),
    lr(None, [SHORTCUT_ERASER_MODE, 0], IconKind::None),
    lr(None, [SHORTCUT_WHITE, 0], IconKind::None),
    lr(None, [SHORTCUT_CLEAR_PAGE, 0], IconKind::Broom),
    lr(None,[SHORTCUT_SHAPES,0],IconKind::Grid),
    lr(None,[SHORTCUT_SHAPE_HOLD,0],IconKind::Clock),
    lr(None,[SHORTCUT_SHAPE_LAST,0],IconKind::Grid),
    lr(None, [SHORTCUT_SELECT, 0], IconKind::Lasso),
];

// TRANSLATORS: (Buttons sheet) the previous and next tool's buttons, in that order (as the other
// rows, about 1100 px at 44 px in bold, with the buttons' names). They go through the pens on the panel.
// TRANSLATORS: (Buttons sheet) the buttons that move the open tool picker's choice to its previous and
// next pen (with it closed, they switch tools through all nine), in that order.
static K_TOOLS: [LearnRow; 11] = [
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_PENCIL.0, 0],
        IconKind::Pencil,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_MECH_PENCIL.0, 0],
        IconKind::MechPencil,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_CHARCOAL.0, 0],
        IconKind::Charcoal,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_BALLPOINT.0, 0],
        IconKind::Ballpoint,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_FINELINER.0, 0],
        IconKind::Fineliner,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_MARKER.0, 0],
        IconKind::Marker,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_FOUNTAIN.0, 0],
        IconKind::Fountain,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_CALLIGRAPHY.0, 0],
        IconKind::Calligraphy,
    ),
    lr(
        None,
        [SHORTCUT_TOOL_BASE + TOOL_HIGHLIGHTER.0, 0],
        IconKind::Highlighter,
    ),
    lr(
        Some(n_("Previous / next tool")),
        [SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL],
        IconKind::Pin,
    ),
    lr(
        Some(n_("Previous / next in the picker")),
        [SHORTCUT_PICKER_PREV, SHORTCUT_PICKER_NEXT],
        IconKind::Grid,
    ),
];

static K_PAGES: [LearnRow; 11] = [
    lr(
        Some(n_("Previous / next page")),
        [SHORTCUT_PREV_PAGE, SHORTCUT_NEXT_PAGE],
        IconKind::Next,
    ),
    lr(
        Some(n_("First / last page")),
        [SHORTCUT_FIRST_PAGE, SHORTCUT_LAST_PAGE],
        IconKind::None,
    ),
    lr(
        Some(n_("Zoom out / zoom in")),
        [SHORTCUT_ZOOM_OUT, SHORTCUT_ZOOM_IN],
        IconKind::Expand,
    ),
    lr(
        Some(n_("Scroll up / down")),
        [SHORTCUT_SCROLL_UP, SHORTCUT_SCROLL_DOWN],
        IconKind::Next,
    ),
    lr(None, [SHORTCUT_NEW_PAGE, 0], IconKind::Plus),
    lr(None, [SHORTCUT_FIT_PAGE, 0], IconKind::Expand),
    lr(None, [SHORTCUT_PAGES, 0], IconKind::Notebook),
    lr(None, [SHORTCUT_LANDSCAPE_FIT, 0], IconKind::Expand),
    lr(None, [SHORTCUT_GESTURE_NEW_PAGE, 0], IconKind::Plus),
    lr(None, [SHORTCUT_PEN_PAN, 0], IconKind::Next),
    lr(None, [SHORTCUT_PEN_ZOOM, 0], IconKind::Expand),
];

static K_INDEX: [LearnRow; 7] = [
    lr(None, [SHORTCUT_LINKS, 0], IconKind::Link),
    lr(None,[SHORTCUT_INDEX_MAP,0],IconKind::Grid),
    lr(None, [SHORTCUT_GO_TO_PAGE, 0], IconKind::Search),
    lr(None, [SHORTCUT_GO_BACK, 0], IconKind::Back),
    lr(None, [SHORTCUT_ADD_LINK, 0], IconKind::LinkPlus),
    lr(None, [SHORTCUT_NAME_PAGE, 0], IconKind::Rename),
    lr(None, [SHORTCUT_LINK_AREAS, 0], IconKind::Eye),
];

static K_MENUS: [LearnRow; 9] = [
    lr(None, [SHORTCUT_TOOL_PICKER, 0], IconKind::Grid),
    lr(None, [SHORTCUT_PAPER, 0], IconKind::Paper),
    lr(None, [SHORTCUT_DISPLAY, 0], IconKind::Brightness),
    lr(None, [SHORTCUT_PEN_FEEL, 0], IconKind::Tune),
    lr(None, [SHORTCUT_FOLD_PANEL, 0], IconKind::Collapse),
    lr(None, [SHORTCUT_LIBRARY, 0], IconKind::Library),
    lr(None, [SHORTCUT_CLOSE_MENU, 0], IconKind::Close),
    lr(None, [SHORTCUT_STATUS, 0], IconKind::Clock),
    lr(None, [SHORTCUT_STATUS_CORNER, 0], IconKind::Clock),
];

static K_SCREEN: [LearnRow; 10] = [
    lr(None, [SHORTCUT_INVERT, 0], IconKind::Invert),
    lr(
        Some(n_("Light down / up")),
        [SHORTCUT_LIGHT_DOWN, SHORTCUT_LIGHT_UP],
        IconKind::Brightness,
    ),
    lr(
        Some(n_("Warmth down / up")),
        [SHORTCUT_WARMTH_DOWN, SHORTCUT_WARMTH_UP],
        IconKind::Brightness,
    ),
    lr(None, [SHORTCUT_ROTATION_LOCK, 0], IconKind::Expand),
    lr(None, [SHORTCUT_PINCH_ZOOM, 0], IconKind::Expand),
    lr(None, [SHORTCUT_THEME, 0], IconKind::Tune),
    lr(None, [SHORTCUT_SIDE_BUTTON, 0], IconKind::Gamepad),
    lr(None, [SHORTCUT_MIRROR_PDF, 0], IconKind::Copy),
    lr(None, [SHORTCUT_SLEEP_SCREEN, 0], IconKind::Exit),
    lr(None, [SHORTCUT_SLEEP, 0], IconKind::Exit),
];

static K_SYSTEM: [LearnRow; 6] = [
    lr(None, [SHORTCUT_GESTURES, 0], IconKind::Grid),
    lr(None, [SHORTCUT_SHORTCUTS, 0], IconKind::Grid),
    lr(None, [SHORTCUT_BLUETOOTH_CONTROLLER, 0], IconKind::Gamepad),
    lr(None, [SHORTCUT_LANGUAGE, 0], IconKind::Language),
    lr(None, [SHORTCUT_SLEEP_AFTER, 0], IconKind::Exit),
    lr(None, [SHORTCUT_UPDATE_CHANNEL, 0], IconKind::Update),
];

static K_GROUPS: [ActionGroup; N_GROUPS] = [
    // TRANSLATORS: the Buttons sheet's list (Settings > Controller > Buttons): its groups of actions,
    // each opening a sheet of them (about 1100 px at 44 px in bold), and those sheets' titles. Editing:
    // undo, redo, the eraser, white ink, clearing the page.
    ActionGroup {
        label: n_("Editing..."),
        title: n_("Editing"),
        icon: IconKind::Undo,
        probe: "editing",
        rows: &K_EDITING,
    },
    // TRANSLATORS: (Buttons sheet) its group of the drawing tools, one by one.
    ActionGroup {
        label: n_("Tools..."),
        title: n_("Tools"),
        icon: IconKind::Grid,
        probe: "tools",
        rows: &K_TOOLS,
    },
    // TRANSLATORS: (Buttons sheet) its group of page turning, new pages, the Pages view and sideways fit.
    ActionGroup {
        label: n_("Pages..."),
        title: n_("Pages"),
        icon: IconKind::Next,
        probe: "pages",
        rows: &K_PAGES,
    },
    // TRANSLATORS: (Buttons sheet) its group of the Index's actions: links, named pages and link areas.
    ActionGroup {
        label: n_("Index..."),
        title: n_("Index"),
        icon: IconKind::Link,
        probe: "index",
        rows: &K_INDEX,
    },
    // TRANSLATORS: (Buttons sheet) its group of the editor's menus: each button opens one (and closes it again).
    ActionGroup {
        label: n_("Menus..."),
        title: n_("Menus"),
        icon: IconKind::More,
        probe: "menus",
        rows: &K_MENUS,
    },
    // TRANSLATORS: (Buttons and Gestures sheets) the screen actions group and its sheet title.
    ActionGroup {
        label: n_("Screen..."),
        title: n_("Screen"),
        icon: IconKind::Brightness,
        probe: "screen",
        rows: &K_SCREEN,
    },
    // TRANSLATORS: (Buttons and Gestures sheets) the system actions group and its sheet title.
    ActionGroup {
        label: n_("System..."),
        title: n_("System"),
        icon: IconKind::Gear,
        probe: "system",
        rows: &K_SYSTEM,
    },
];

pub const ACTION_GROUPS: [ActionGroup; N_GROUPS] = K_GROUPS;

// ---- action names and metadata ----

/// The old names stay in their original lookup order; new phase 3 names follow the old set.
pub static K_ACTIONS: [(&str, i32); 78] = [
    ("undo", SHORTCUT_UNDO),
    ("redo", SHORTCUT_REDO),
    ("previous-page", SHORTCUT_PREV_PAGE),
    ("next-page", SHORTCUT_NEXT_PAGE),
    ("previous-tool", SHORTCUT_PREV_TOOL),
    ("next-tool", SHORTCUT_NEXT_TOOL),
    ("white", SHORTCUT_WHITE),
    ("hold-eraser", SHORTCUT_HOLD_ERASER),
    ("eraser-mode", SHORTCUT_ERASER_MODE),
    ("clear-page", SHORTCUT_CLEAR_PAGE),
    ("first-page", SHORTCUT_FIRST_PAGE),
    ("last-page", SHORTCUT_LAST_PAGE),
    ("new-page", SHORTCUT_NEW_PAGE),
    ("fit-page", SHORTCUT_FIT_PAGE),
    ("pages", SHORTCUT_PAGES),
    ("tool-picker", SHORTCUT_TOOL_PICKER),
    ("paper", SHORTCUT_PAPER),
    ("display", SHORTCUT_DISPLAY),
    ("pen-feel", SHORTCUT_PEN_FEEL),
    ("fold-panel", SHORTCUT_FOLD_PANEL),
    ("library", SHORTCUT_LIBRARY),
    ("close-menu", SHORTCUT_CLOSE_MENU),
    ("picker-previous", SHORTCUT_PICKER_PREV),
    ("picker-next", SHORTCUT_PICKER_NEXT),
    ("add-link", SHORTCUT_ADD_LINK),
    ("name-page", SHORTCUT_NAME_PAGE),
    ("status-toggle", SHORTCUT_STATUS),
    ("links", SHORTCUT_LINKS),
    ("go-to-page", SHORTCUT_GO_TO_PAGE),
    ("go-back", SHORTCUT_GO_BACK),
    ("link-areas", SHORTCUT_LINK_AREAS),
    ("landscape-fit", SHORTCUT_LANDSCAPE_FIT),
    ("status-corner", SHORTCUT_STATUS_CORNER),
    ("gesture-new-page", SHORTCUT_GESTURE_NEW_PAGE),
    ("zoom-out", SHORTCUT_ZOOM_OUT),
    ("zoom-in", SHORTCUT_ZOOM_IN),
    ("scroll-up", SHORTCUT_SCROLL_UP),
    ("scroll-down", SHORTCUT_SCROLL_DOWN),
    ("invert", SHORTCUT_INVERT),
    ("light-down", SHORTCUT_LIGHT_DOWN),
    ("light-up", SHORTCUT_LIGHT_UP),
    ("warmth-down", SHORTCUT_WARMTH_DOWN),
    ("warmth-up", SHORTCUT_WARMTH_UP),
    ("rotation-lock", SHORTCUT_ROTATION_LOCK),
    ("pinch-zoom", SHORTCUT_PINCH_ZOOM),
    ("theme", SHORTCUT_THEME),
    ("side-button", SHORTCUT_SIDE_BUTTON),
    ("mirror-pdf", SHORTCUT_MIRROR_PDF),
    ("sleep-screen", SHORTCUT_SLEEP_SCREEN),
    ("sleep", SHORTCUT_SLEEP),
    ("gestures", SHORTCUT_GESTURES),
    ("shortcuts", SHORTCUT_SHORTCUTS),
    ("bluetooth-controller", SHORTCUT_BLUETOOTH_CONTROLLER),
    ("language", SHORTCUT_LANGUAGE),
    ("sleep-after", SHORTCUT_SLEEP_AFTER),
    ("update-channel", SHORTCUT_UPDATE_CHANNEL),
    ("shapes",SHORTCUT_SHAPES),("shape-hold",SHORTCUT_SHAPE_HOLD),
    ("shape-last",SHORTCUT_SHAPE_LAST),("index-map",SHORTCUT_INDEX_MAP),
    ("pan", SHORTCUT_PEN_PAN), ("zoom", SHORTCUT_PEN_ZOOM),
    ("select", SHORTCUT_SELECT),
    ("select-exact", SHORTCUT_SELECT_EXACT),
    ("new-pages", SHORTCUT_NEW_PAGES),
    ("save", SHORTCUT_SAVE),
    ("back", SHORTCUT_BACK),
    ("two-finger-up", SHORTCUT_TWO_FINGER_UP),
    ("new-notebook", SHORTCUT_NEW_NOTEBOOK),
    ("new-folder", SHORTCUT_NEW_FOLDER),
    ("share", SHORTCUT_SHARE),
    ("settings", SHORTCUT_SETTINGS),
    ("select-all", SHORTCUT_SELECT_ALL),
    ("graph-view", SHORTCUT_GRAPH_VIEW),
    ("pan-left", SHORTCUT_PAN_LEFT),
    ("pan-right", SHORTCUT_PAN_RIGHT),
    ("pan-up", SHORTCUT_PAN_UP),
    ("pan-down", SHORTCUT_PAN_DOWN),
];

const TOOL_NAMES: [(&str, i32); TOOL_COUNT] = [
    ("tool:pencil", SHORTCUT_TOOL_BASE),
    ("tool:mech", SHORTCUT_TOOL_BASE + 1),
    ("tool:ballpoint", SHORTCUT_TOOL_BASE + 2),
    ("tool:eraser", SHORTCUT_TOOL_BASE + 3),
    ("tool:highlighter", SHORTCUT_TOOL_BASE + 4),
    ("tool:fountain", SHORTCUT_TOOL_BASE + 5),
    ("tool:fineliner", SHORTCUT_TOOL_BASE + 6),
    ("tool:marker", SHORTCUT_TOOL_BASE + 7),
    ("tool:charcoal", SHORTCUT_TOOL_BASE + 8),
    ("tool:calligraphy", SHORTCUT_TOOL_BASE + 9),
];

static ACTIONS: [ActionInfo; 88] = [
    // Tools are action ids too, even though the tool-specific labels retain msgctxt "tool".
    ai(
        SHORTCUT_TOOL_BASE,
        "tool:pencil",
        nc_("tool", "Pencil"),
        Some("tool"),
        IconKind::Pencil,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 1,
        "tool:mech",
        nc_("tool", "Mechanical pencil"),
        Some("tool"),
        IconKind::MechPencil,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 2,
        "tool:ballpoint",
        nc_("tool", "Ballpoint"),
        Some("tool"),
        IconKind::Ballpoint,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 3,
        "tool:eraser",
        nc_("tool", "Eraser"),
        Some("tool"),
        IconKind::Eraser,
        GROUP_EDITING,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 4,
        "tool:highlighter",
        nc_("tool", "Highlighter"),
        Some("tool"),
        IconKind::Highlighter,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 5,
        "tool:fountain",
        nc_("tool", "Fountain pen"),
        Some("tool"),
        IconKind::Fountain,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 6,
        "tool:fineliner",
        nc_("tool", "Fineliner"),
        Some("tool"),
        IconKind::Fineliner,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 7,
        "tool:marker",
        nc_("tool", "Marker"),
        Some("tool"),
        IconKind::Marker,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 8,
        "tool:charcoal",
        nc_("tool", "Charcoal"),
        Some("tool"),
        IconKind::Charcoal,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    ai(
        SHORTCUT_TOOL_BASE + 9,
        "tool:calligraphy",
        nc_("tool", "Calligraphy pen"),
        Some("tool"),
        IconKind::Calligraphy,
        GROUP_TOOLS,
        0,
        ActionKind::Cycle,
        SRC_ALL,
    ),
    // Existing actions. Translator notes and msgids from library.rs are retained at their moved labels.
    nm(
        SHORTCUT_UNDO,
        "undo",
        n_("Undo"),
        GROUP_EDITING,
        IconKind::Undo,
        ActionKind::OneShot,
    ),
    nm(
        SHORTCUT_REDO,
        "redo",
        n_("Redo"),
        GROUP_EDITING,
        IconKind::Redo,
        ActionKind::OneShot,
    ),
    pair(
        SHORTCUT_PREV_PAGE,
        "previous-page",
        n_("Previous page"),
        GROUP_PAGES,
        IconKind::Prev,
        SHORTCUT_NEXT_PAGE,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_NEXT_PAGE,
        "next-page",
        n_("Next page"),
        GROUP_PAGES,
        IconKind::Next,
        SHORTCUT_PREV_PAGE,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_PREV_TOOL,
        "previous-tool",
        n_("Previous tool"),
        GROUP_TOOLS,
        IconKind::Prev,
        SHORTCUT_NEXT_TOOL,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_NEXT_TOOL,
        "next-tool",
        n_("Next tool"),
        GROUP_TOOLS,
        IconKind::Next,
        SHORTCUT_PREV_TOOL,
        ActionKind::Step,
    ),
    nm(
        SHORTCUT_WHITE,
        "white",
        n_("White ink"),
        GROUP_EDITING,
        IconKind::None,
        ActionKind::Toggle,
    ),
    ai(
        SHORTCUT_HOLD_ERASER,
        "hold-eraser",
        n_("Erase while held"),
        None,
        IconKind::Gamepad,
        GROUP_EDITING,
        0,
        ActionKind::Momentary,
        SRC_BUTTONS_PEN,
    ),
    nm(
        SHORTCUT_ERASER_MODE,
        "eraser-mode",
        n_("Eraser mode"),
        GROUP_EDITING,
        IconKind::None,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_CLEAR_PAGE,
        "clear-page",
        n_("Clear page"),
        GROUP_EDITING,
        IconKind::Broom,
        ActionKind::OneShot,
    ),
    pair(
        SHORTCUT_FIRST_PAGE,
        "first-page",
        n_("First page"),
        GROUP_PAGES,
        IconKind::Prev,
        SHORTCUT_LAST_PAGE,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_LAST_PAGE,
        "last-page",
        n_("Last page"),
        GROUP_PAGES,
        IconKind::Next,
        SHORTCUT_FIRST_PAGE,
        ActionKind::Step,
    ),
    nm(
        SHORTCUT_NEW_PAGE,
        "new-page",
        n_("New page"),
        GROUP_PAGES,
        IconKind::Plus,
        ActionKind::OneShot,
    ),
    nm(
        SHORTCUT_FIT_PAGE,
        "fit-page",
        n_("Fit page"),
        GROUP_PAGES,
        IconKind::Expand,
        ActionKind::OneShot,
    ),
    nm(
        SHORTCUT_PAGES,
        "pages",
        n_("Pages"),
        GROUP_PAGES,
        IconKind::Notebook,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_TOOL_PICKER,
        "tool-picker",
        n_("Tool picker"),
        GROUP_MENUS,
        IconKind::Grid,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_PAPER,
        "paper",
        n_("Paper"),
        GROUP_MENUS,
        IconKind::Paper,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_DISPLAY,
        "display",
        n_("Display"),
        GROUP_MENUS,
        IconKind::Brightness,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_PEN_FEEL,
        "pen-feel",
        n_("Pen feel"),
        GROUP_MENUS,
        IconKind::Tune,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_FOLD_PANEL,
        "fold-panel",
        n_("Fold the toolbar"),
        GROUP_MENUS,
        IconKind::Collapse,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_LIBRARY,
        "library",
        n_("Library"),
        GROUP_MENUS,
        IconKind::Library,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_CLOSE_MENU,
        "close-menu",
        n_("Close menu"),
        GROUP_MENUS,
        IconKind::Close,
        ActionKind::OneShot,
    ),
    nm(
        SHORTCUT_PICKER_PREV,
        "picker-previous",
        n_("Previous in the picker"),
        GROUP_TOOLS,
        IconKind::Prev,
        ActionKind::Step,
    ),
    nm(
        SHORTCUT_PICKER_NEXT,
        "picker-next",
        n_("Next in the picker"),
        GROUP_TOOLS,
        IconKind::Next,
        ActionKind::Step,
    ),
    // TRANSLATORS: (Buttons sheet) starts a link on the open page, as the Index's Add a link does.
    ai(
        SHORTCUT_ADD_LINK,
        "add-link",
        nc_("buttons", "Add a link"),
        Some("buttons"),
        IconKind::LinkPlus,
        GROUP_INDEX,
        0,
        ActionKind::Menu,
        SRC_ALL,
    ),
    // TRANSLATORS: (Buttons sheet) the naming keyboard for the open page, as the Index's Name this page opens it.
    nm(
        SHORTCUT_NAME_PAGE,
        "name-page",
        n_("Name the page"),
        GROUP_INDEX,
        IconKind::Rename,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_STATUS,
        "status-toggle",
        n_("Clock and battery"),
        GROUP_MENUS,
        IconKind::Clock,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_LINKS,
        "links",
        n_("Links on this page"),
        GROUP_INDEX,
        IconKind::Link,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_GO_TO_PAGE,
        "go-to-page",
        n_("Go to page"),
        GROUP_INDEX,
        IconKind::Search,
        ActionKind::Menu,
    ),
    nm(
        SHORTCUT_GO_BACK,
        "go-back",
        n_("Back to the page before"),
        GROUP_INDEX,
        IconKind::Back,
        ActionKind::OneShot,
    ),
    nm(
        SHORTCUT_LINK_AREAS,
        "link-areas",
        n_("Show link areas"),
        GROUP_INDEX,
        IconKind::Eye,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_LANDSCAPE_FIT,
        "landscape-fit",
        n_("Whole page sideways"),
        GROUP_PAGES,
        IconKind::Expand,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_STATUS_CORNER,
        "status-corner",
        n_("Clock and battery corner"),
        GROUP_MENUS,
        IconKind::Clock,
        ActionKind::Cycle,
    ),
    nm(
        SHORTCUT_GESTURE_NEW_PAGE,
        "gesture-new-page",
        n_("Swiping past the last page adds a page"),
        GROUP_PAGES,
        IconKind::Plus,
        ActionKind::Toggle,
    ),
    // New phase 3 actions, ids 56 to 77.
    pair(
        SHORTCUT_ZOOM_OUT,
        "zoom-out",
        n_("Zoom out"),
        GROUP_PAGES,
        IconKind::Minus,
        SHORTCUT_ZOOM_IN,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_ZOOM_IN,
        "zoom-in",
        n_("Zoom in"),
        GROUP_PAGES,
        IconKind::Expand,
        SHORTCUT_ZOOM_OUT,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_SCROLL_UP,
        "scroll-up",
        n_("Scroll up"),
        GROUP_PAGES,
        IconKind::Up,
        SHORTCUT_SCROLL_DOWN,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_SCROLL_DOWN,
        "scroll-down",
        n_("Scroll down"),
        GROUP_PAGES,
        IconKind::Down,
        SHORTCUT_SCROLL_UP,
        ActionKind::Step,
    ),
    nm(
        SHORTCUT_INVERT,
        "invert",
        n_("Invert colors"),
        GROUP_SCREEN,
        IconKind::Invert,
        ActionKind::Toggle,
    ),
    pair(
        SHORTCUT_LIGHT_DOWN,
        "light-down",
        n_("Less light"),
        GROUP_SCREEN,
        IconKind::Minus,
        SHORTCUT_LIGHT_UP,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_LIGHT_UP,
        "light-up",
        n_("More light"),
        GROUP_SCREEN,
        IconKind::Plus,
        SHORTCUT_LIGHT_DOWN,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_WARMTH_DOWN,
        "warmth-down",
        n_("Less warmth"),
        GROUP_SCREEN,
        IconKind::Minus,
        SHORTCUT_WARMTH_UP,
        ActionKind::Step,
    ),
    pair(
        SHORTCUT_WARMTH_UP,
        "warmth-up",
        n_("More warmth"),
        GROUP_SCREEN,
        IconKind::Plus,
        SHORTCUT_WARMTH_DOWN,
        ActionKind::Step,
    ),
    nm(
        SHORTCUT_ROTATION_LOCK,
        "rotation-lock",
        n_("Lock rotation"),
        GROUP_SCREEN,
        IconKind::Expand,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_PINCH_ZOOM,
        "pinch-zoom",
        n_("Pinch to zoom"),
        GROUP_SCREEN,
        IconKind::Expand,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_THEME,
        "theme",
        n_("Switch theme"),
        GROUP_SCREEN,
        IconKind::Tune,
        ActionKind::Cycle,
    ),
    nm(
        SHORTCUT_SIDE_BUTTON,
        "side-button",
        n_("Side button"),
        GROUP_SCREEN,
        IconKind::Gamepad,
        ActionKind::Cycle,
    ),
    nm(
        SHORTCUT_MIRROR_PDF,
        "mirror-pdf",
        n_("PDF copies"),
        GROUP_SCREEN,
        IconKind::Copy,
        ActionKind::Toggle,
    ),
    nm(
        SHORTCUT_SLEEP_SCREEN,
        "sleep-screen",
        n_("Sleep screen"),
        GROUP_SCREEN,
        IconKind::Exit,
        ActionKind::Cycle,
    ),
    nm(
        SHORTCUT_SLEEP,
        "sleep",
        n_("Sleep now"),
        GROUP_SCREEN,
        IconKind::Exit,
        ActionKind::OneShot,
    ),
    nm(
        SHORTCUT_GESTURES,
        "gestures",
        n_("Gestures"),
        GROUP_SYSTEM,
        IconKind::Grid,
        ActionKind::Toggle,
    ),
    ai(
        SHORTCUT_SHORTCUTS,
        "shortcuts",
        n_("Enable shortcuts"),
        None,
        IconKind::Grid,
        GROUP_SYSTEM,
        0,
        ActionKind::Toggle,
        SRC_GESTURES,
    ),
    ai(
        SHORTCUT_BLUETOOTH_CONTROLLER,
        "bluetooth-controller",
        n_("Bluetooth controller"),
        None,
        IconKind::Gamepad,
        GROUP_SYSTEM,
        0,
        ActionKind::Toggle,
        SRC_GESTURES,
    ),
    nm(
        SHORTCUT_LANGUAGE,
        "language",
        n_("Language"),
        GROUP_SYSTEM,
        IconKind::Language,
        ActionKind::Cycle,
    ),
    nm(
        SHORTCUT_SLEEP_AFTER,
        "sleep-after",
        n_("Sleep after"),
        GROUP_SYSTEM,
        IconKind::Exit,
        ActionKind::Cycle,
    ),
    nm(
        SHORTCUT_UPDATE_CHANNEL,
        "update-channel",
        n_("Update channel"),
        GROUP_SYSTEM,
        IconKind::Update,
        ActionKind::Cycle,
    ),
    nm(SHORTCUT_SHAPES,"shapes",n_("Smart shapes"),GROUP_EDITING,IconKind::Grid,ActionKind::Toggle),
    nm(SHORTCUT_SHAPE_HOLD,"shape-hold",n_("Shape hold time"),GROUP_EDITING,IconKind::Clock,ActionKind::Cycle),
    nm(SHORTCUT_SHAPE_LAST,"shape-last",n_("Shape the last stroke"),GROUP_EDITING,IconKind::Grid,ActionKind::OneShot),
    nm(SHORTCUT_INDEX_MAP,"index-map",n_("Graph"),GROUP_INDEX,IconKind::Grid,ActionKind::Menu),
    ai(
        SHORTCUT_PEN_PAN, "pan", n_("Pan with the pen"), None, IconKind::Next,
        GROUP_PAGES, 0, ActionKind::Momentary, SRC_PEN,
    ),
    ai(
        SHORTCUT_PEN_ZOOM, "zoom", n_("Zoom with the pen"), None, IconKind::Expand,
        GROUP_PAGES, 0, ActionKind::Momentary, SRC_PEN,
    ),
    ai(
        // TRANSLATORS: (Buttons sheet) Select mode on or off: the pen draws a lasso around ink to copy, cut,
        // paste or move it.
        SHORTCUT_SELECT, "select", n_("Select (lasso)"), None, IconKind::Lasso,
        GROUP_EDITING, 0, ActionKind::Toggle, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (Gestures and shortcuts lists, about 1100 px at 44 px in bold) Select mode with a lasso
        // that cuts strokes along its line (exact area); with Select mode on, it switches between whole
        // strokes and exact area.
        SHORTCUT_SELECT_EXACT, "select-exact", n_("Select exact area"), None, IconKind::Lasso,
        GROUP_EDITING, 0, ActionKind::Toggle, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (Gestures and shortcuts lists, about 1100 px at 44 px in bold) opens the count pad that
        // adds several blank pages at once after the open page, which stays open.
        SHORTCUT_NEW_PAGES, "new-pages", n_("New pages..."), None, IconKind::Plus,
        GROUP_PAGES, 0, ActionKind::OneShot, SRC_ALL,
    ),
    // The keyboard scheme of 2.0.0-beta.2 (ids 87 to 93). None is in a Buttons sheet group (those learn a
    // controller's buttons, and are full): they are bound in shortcuts.ini.
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) Ctrl+S: the open page and
        // its notebook written now (in the Library, any setting waiting).
        SHORTCUT_SAVE, "save", n_("Save now"), None, IconKind::Check,
        GROUP_MENUS, 0, ActionKind::OneShot, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) Esc: what is in front closes
        // (a dialog, a menu, a selection), else back to the Library, and there "Exit Grimoire?".
        SHORTCUT_BACK, "back", n_("Back or close"), None, IconKind::Back,
        GROUP_MENUS, 0, ActionKind::OneShot, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) Space on a page: does what
        // two fingers swiped up do (at first, the Links bar). (No gesture does it: it is a gesture's own.)
        SHORTCUT_TWO_FINGER_UP, "two-finger-up", n_("What two fingers up do"), None, IconKind::Link,
        GROUP_INDEX, 0, ActionKind::OneShot, SRC_BUTTONS_PEN,
    ),
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) Ctrl+N in the Library: the
        // New notebook sheet, as the top bar's New button.
        SHORTCUT_NEW_NOTEBOOK, "new-notebook", n_("New notebook"), None, IconKind::Plus,
        GROUP_MENUS, 0, ActionKind::Menu, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) Ctrl+F in the Library: a new
        // folder's name keyboard, as the top bar's Folder button.
        SHORTCUT_NEW_FOLDER, "new-folder", n_("New folder"), None, IconKind::FolderPlus,
        GROUP_MENUS, 0, ActionKind::Menu, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) S in the Library: the
        // Share screen, as the top bar's Share button.
        SHORTCUT_SHARE, "share", n_("Share"), None, IconKind::Share,
        GROUP_MENUS, 0, ActionKind::Menu, SRC_ALL,
    ),
    ai(
        // TRANSLATORS: (the keyboard shortcuts lists, about 1100 px at 44 px in bold) O in the Library: the
        // Settings sheet, as the top bar's Settings button.
        SHORTCUT_SETTINGS, "settings", n_("Settings"), None, IconKind::Gear,
        GROUP_MENUS, 0, ActionKind::Menu, SRC_ALL,
    ),
    // select-all acts in the Pages view and the Graph (its K_PLACES row); not on a page.
    ai(
        // TRANSLATORS: (the Keyboard shortcuts list, about 1100 px at 44 px in bold) ticks every page in the Pages
        // view (and selects every node in the Graph): the same words as the Pages view's Select all button.
        SHORTCUT_SELECT_ALL, "select-all", n_("Select all"), None, IconKind::Check,
        GROUP_PAGES, 0, ActionKind::OneShot, SRC_ALL,
    ),
    // The Graph's keys: in no Buttons sheet group (those learn controller buttons); bound in
    // shortcuts.ini (keys and a controller's buttons; the pen's, as every action).
    ai(
        // TRANSLATORS: (shortcuts lists, about 1100 px at 44 px in bold) in the Graph, its view button's action:
        // View: All and View: Adjacent in turn (all notebooks' pages, or the pages around one).
        SHORTCUT_GRAPH_VIEW, "graph-view", n_("Graph view: All or Adjacent"), None, IconKind::Grid,
        GROUP_INDEX, 0, ActionKind::Cycle, SRC_BUTTONS_PEN,
    ),
    ai(
        // TRANSLATORS: (shortcuts lists, about 1100 px at 44 px in bold) in the Graph, the left arrow: the view moves
        // left (the map slides the other way). Right, up and down follow.
        SHORTCUT_PAN_LEFT, "pan-left", n_("Move the Graph left"), None, IconKind::Next,
        GROUP_INDEX, SHORTCUT_PAN_RIGHT, ActionKind::Step, SRC_BUTTONS_PEN,
    ),
    ai(
        // TRANSLATORS: (shortcuts lists, about 1100 px at 44 px in bold) in the Graph, the right arrow: the view moves
        // right (the map slides the other way).
        SHORTCUT_PAN_RIGHT, "pan-right", n_("Move the Graph right"), None, IconKind::Next,
        GROUP_INDEX, SHORTCUT_PAN_LEFT, ActionKind::Step, SRC_BUTTONS_PEN,
    ),
    ai(
        // TRANSLATORS: (shortcuts lists, about 1100 px at 44 px in bold) in the Graph, the up arrow: the view moves
        // up (the map slides the other way).
        SHORTCUT_PAN_UP, "pan-up", n_("Move the Graph up"), None, IconKind::Next,
        GROUP_INDEX, SHORTCUT_PAN_DOWN, ActionKind::Step, SRC_BUTTONS_PEN,
    ),
    ai(
        // TRANSLATORS: (shortcuts lists, about 1100 px at 44 px in bold) in the Graph, the down arrow: the view moves
        // down (the map slides the other way).
        SHORTCUT_PAN_DOWN, "pan-down", n_("Move the Graph down"), None, IconKind::Next,
        GROUP_INDEX, SHORTCUT_PAN_UP, ActionKind::Step, SRC_BUTTONS_PEN,
    ),
];

pub fn action_info(id: i32) -> Option<&'static ActionInfo> {
    ACTIONS.iter().find(|a| a.id == id)
}

pub fn action_by_name(name: &str) -> i32 {
    if name == "status-mode" { return SHORTCUT_STATUS; } // retired display-mode binding
    if name == "index" { return SHORTCUT_INDEX_MAP; } // retired Index list binding: the Graph now
    for &(n, id) in TOOL_NAMES.iter() {
        if n == name {
            return id;
        }
    }
    for &(n, id) in K_ACTIONS.iter() {
        if n == name {
            return id;
        }
    }
    SHORTCUT_NONE
}

/// The registry's translated name for a row, preserving the existing contexts used by the Buttons sheet.
pub fn action_label(id: i32) -> &'static str {
    let Some(info) = action_info(id) else {
        return "?";
    };
    match info.ctx {
        Some(ctx) => c_(ctx, info.label),
        None => tr(info.label),
    }
}

pub fn action_sources(id: i32) -> u8 {
    action_info(id).map_or(0, |a| a.sources)
}

// ---- places ----
//
// A key can mean one thing in one place and another elsewhere (Ctrl+N: New notebook in the Library, New page on a
// notebook page). Each action acts in its places; a key press is looked up for the place it was pressed in
// (app_place), the last line for that key whose action acts there winning (shortcuts_lookup_at).

/// The Library (and the Share screen, where only `back` acts).
pub const PL_LIBRARY: u8 = 1;
/// A notebook page in the editor, menus over it included.
pub const PL_PAGE: u8 = 2;
/// The Pages view (the unified Index's tabs included).
pub const PL_PAGES: u8 = 4;
/// The Graph, from the Library or from a notebook.
pub const PL_GRAPH: u8 = 8;
pub const PL_PENFEEL: u8 = 16;
pub const PL_ALL: u8 = 31;

/// Every action's places but those that act on a page alone (an action not here: PL_PAGE). New actions add
/// their rows at the end, in id order.
static K_PLACES: &[(i32, u8)] = &[
    (SHORTCUT_TOOL_BASE, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 1, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 2, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 3, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 4, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 5, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 6, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 7, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 8, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_TOOL_BASE + 9, PL_PAGE | PL_PENFEEL),
    (SHORTCUT_UNDO, PL_PAGE | PL_PAGES),
    (SHORTCUT_REDO, PL_PAGE | PL_PAGES),
    (SHORTCUT_PREV_PAGE, PL_PAGE | PL_PAGES | PL_LIBRARY),
    (SHORTCUT_NEXT_PAGE, PL_PAGE | PL_PAGES | PL_LIBRARY),
    (SHORTCUT_NEW_PAGE, PL_PAGE | PL_PAGES),
    (SHORTCUT_FIT_PAGE, PL_PAGE | PL_GRAPH),
    // The editor's menus' buttons reach its Pages view and its Graph too, which are menus over the page: there
    // each closes the one in front and opens its own, as a controller's did before places (Pages over the
    // Graph turns it to Pages; Library leaves). A controller's Pen feel button closes Pen feel, and Close menu
    // closes what each place has, as before.
    (SHORTCUT_PAGES, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_TOOL_PICKER, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_PAPER, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_DISPLAY, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_PEN_FEEL, PL_PAGE | PL_PAGES | PL_GRAPH | PL_PENFEEL),
    (SHORTCUT_LIBRARY, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_LINKS, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_CLOSE_MENU, PL_ALL),
    (SHORTCUT_STATUS, PL_ALL),
    (SHORTCUT_GO_TO_PAGE, PL_PAGE | PL_PAGES | PL_GRAPH),
    (SHORTCUT_GO_BACK, PL_PAGE | PL_GRAPH),
    (SHORTCUT_STATUS_CORNER, PL_ALL),
    (SHORTCUT_ZOOM_OUT, PL_PAGE | PL_GRAPH),
    (SHORTCUT_ZOOM_IN, PL_PAGE | PL_GRAPH),
    (SHORTCUT_INVERT, PL_ALL),
    (SHORTCUT_LIGHT_DOWN, PL_ALL),
    (SHORTCUT_LIGHT_UP, PL_ALL),
    (SHORTCUT_WARMTH_DOWN, PL_ALL),
    (SHORTCUT_WARMTH_UP, PL_ALL),
    (SHORTCUT_ROTATION_LOCK, PL_ALL),
    (SHORTCUT_PINCH_ZOOM, PL_ALL),
    (SHORTCUT_THEME, PL_ALL),
    (SHORTCUT_SIDE_BUTTON, PL_ALL),
    (SHORTCUT_MIRROR_PDF, PL_ALL),
    (SHORTCUT_SLEEP_SCREEN, PL_ALL),
    (SHORTCUT_SLEEP, PL_ALL),
    (SHORTCUT_GESTURES, PL_ALL),
    (SHORTCUT_SHORTCUTS, PL_ALL),
    (SHORTCUT_BLUETOOTH_CONTROLLER, PL_ALL),
    (SHORTCUT_LANGUAGE, PL_ALL),
    (SHORTCUT_SLEEP_AFTER, PL_ALL),
    (SHORTCUT_UPDATE_CHANNEL, PL_ALL),
    (SHORTCUT_INDEX_MAP, PL_PAGE | PL_PAGES | PL_LIBRARY | PL_GRAPH),
    // The Pages view takes New pages too (Ctrl+N's count pad, as new-page).
    (SHORTCUT_NEW_PAGES, PL_PAGE | PL_PAGES),
    (SHORTCUT_SAVE, PL_ALL),
    (SHORTCUT_BACK, PL_ALL),
    (SHORTCUT_TWO_FINGER_UP, PL_PAGE),
    (SHORTCUT_NEW_NOTEBOOK, PL_LIBRARY),
    (SHORTCUT_NEW_FOLDER, PL_LIBRARY),
    (SHORTCUT_SHARE, PL_LIBRARY),
    (SHORTCUT_SETTINGS, PL_LIBRARY),
    (SHORTCUT_SELECT_ALL, PL_PAGES | PL_GRAPH),
    (SHORTCUT_GRAPH_VIEW, PL_GRAPH),
    (SHORTCUT_PAN_LEFT, PL_GRAPH),
    (SHORTCUT_PAN_RIGHT, PL_GRAPH),
    (SHORTCUT_PAN_UP, PL_GRAPH),
    (SHORTCUT_PAN_DOWN, PL_GRAPH),
];

/// The places an action acts in (PL_* bits): K_PLACES's, else a notebook page alone.
pub fn action_places(id: i32) -> u8 {
    K_PLACES.iter().find(|&&(a, _)| a == id).map_or(PL_PAGE, |&(_, p)| p)
}

/// Where a key press is now, one PL_* bit: the Library (with its Graph shown, the Graph; the Share and Calibrate
/// screens count as the Library), a notebook (its Graph, its Pages view, else the page, whatever menu is over
/// it), or Pen feel.
pub fn app_place(a: &App) -> u8 {
    match a.scr {
        ScreenId::Library if crate::library::library_graph_shown(a) => PL_GRAPH,
        ScreenId::Library | ScreenId::Share | ScreenId::Calib => PL_LIBRARY,
        ScreenId::Penset => PL_PENFEEL,
        ScreenId::Editor if crate::library::library_settings_active(a) => PL_PAGE,
        ScreenId::Editor => crate::editor::editor_place(a),
    }
}

pub fn group_row_count(group: usize) -> usize {
    ACTION_GROUPS.get(group).map_or(0, |g| g.rows.len())
}

pub fn learn_row_at(group: usize, row: usize) -> Option<LearnRow> {
    ACTION_GROUPS.get(group)?.rows.get(row).copied()
}

const _: () = {
    assert!(K_EDITING.len() <= GROUP_ROWS_MAX);
    assert!(K_TOOLS.len() <= GROUP_ROWS_MAX);
    assert!(K_PAGES.len() <= GROUP_ROWS_MAX);
    assert!(K_INDEX.len() <= GROUP_ROWS_MAX);
    assert!(K_MENUS.len() <= GROUP_ROWS_MAX);
    assert!(K_SCREEN.len() <= GROUP_ROWS_MAX);
    assert!(K_SYSTEM.len() <= GROUP_ROWS_MAX);
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every action label from the keyboard scheme of 2.0.0-beta.2 on (ids from 87) has a TRANSLATORS comment
    /// above it in ACTIONS: the label's width budget in the keyboard shortcuts lists, and what it does.
    #[test]
    fn new_action_labels_have_translators_comments() {
        let src = include_str!("actions.rs");
        let table = src.split_once("static ACTIONS").unwrap().1.split_once("\n];").unwrap().0;
        let mut checked = 0;
        for entry in table.split("    ai(").skip(1) {
            let code = entry.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
            let Some(name) = code.split('"').nth(1) else { continue };
            if action_by_name(name) >= 87 {
                assert!(entry.contains("// TRANSLATORS: ("), "{name}: no TRANSLATORS comment with its budget");
                checked += 1;
            }
        }
        assert!(checked >= 7, "{checked}");
    }

    /// Ids are added to these tables in several places (merges have collided before): no id and no name twice
    /// across ACTIONS, K_ACTIONS and TOOL_NAMES, every name naming its own id, every action in K_PLACES once.
    #[test]
    fn no_action_id_or_name_twice() {
        let mut ids: Vec<i32> = ACTIONS.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "an action id is in ACTIONS twice");
        let mut names: Vec<&str> = ACTIONS.iter().map(|a| a.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "an action name is in ACTIONS twice");
        let mut listed: Vec<(&str, i32)> = K_ACTIONS.iter().chain(TOOL_NAMES.iter()).copied().collect();
        let m = listed.len();
        listed.sort_unstable();
        listed.dedup_by(|a, b| a.0 == b.0);
        assert_eq!(listed.len(), m, "a name is in K_ACTIONS or TOOL_NAMES twice");
        let mut by_id: Vec<i32> = listed.iter().map(|&(_, id)| id).collect();
        by_id.sort_unstable();
        by_id.dedup();
        assert_eq!(by_id.len(), m, "two names in K_ACTIONS or TOOL_NAMES give one id");
        assert_eq!(m, n, "ACTIONS and K_ACTIONS with TOOL_NAMES list different actions");
        for (name, id) in listed {
            let info = action_info(id).unwrap_or_else(|| panic!("{name}: id {id} is not in ACTIONS"));
            assert_eq!(info.name, name, "id {id}");
            assert_eq!(action_by_name(name), id, "{name}");
        }
        let mut placed: Vec<i32> = K_PLACES.iter().map(|&(id, _)| id).collect();
        placed.sort_unstable();
        let k = placed.len();
        placed.dedup();
        assert_eq!(placed.len(), k, "an action is in K_PLACES twice");
        for id in placed {
            assert!(action_info(id).is_some(), "K_PLACES names id {id}, no action");
        }
    }

    #[test]
    fn places_of_the_scheme() {
        assert_eq!(action_places(SHORTCUT_NEW_NOTEBOOK), PL_LIBRARY);
        assert_eq!(action_places(SHORTCUT_NEW_PAGE), PL_PAGE | PL_PAGES);
        assert_eq!(action_places(SHORTCUT_BACK), PL_ALL);
        assert_eq!(action_places(SHORTCUT_SELECT), PL_PAGE); // (not listed: a page's)
        assert_eq!(action_places(SHORTCUT_TOOL_BASE + TOOL_ERASER.0) & PL_PENFEEL, PL_PENFEEL);
        assert_eq!(action_places(SHORTCUT_INVERT), PL_ALL);
        assert_eq!(action_places(SHORTCUT_INDEX_MAP), PL_ALL & !PL_PENFEEL);
    }
}
