//! Settings' categories and rows as data: the backend of the Settings page.
//!
//! Adding a row is one `Item` line in the table below, picked by what the row does:
//!
//! * `Item::open(icon, label, id)` — a row that opens a page or a sheet;
//! * `Item::toggle(icon, label, id, check)` — an on/off row with its switch at the right;
//! * `Item::value(icon, label, id)` — a row whose label is built from the current setting
//!   (`Label::Style`, `Label::Font`, ...), and `Item::value_toggle` for one with a switch too;
//! * `.classic_only()` — a row Gothic hides; `.when(Show::Wifi)` — one a platform that leaves
//!   that service to its system leaves out (`Show`, light.rs's `Services`).
//!
//! Then add its `SettingId` to `library::setting_hit` (and its action to `library::act` if it is a
//! new kind of action). Nothing draws the row itself: `library::draw_settings` lays the list out
//! with `ui::ui_row_page` and draws every rect with `ui::ui_row`, the one row design in the app, so
//! a new item looks like every other one and the page fits or pages a longer list by itself.
//!
//! Adding a category is one `Category::new(...)` line in `CATEGORIES`: the sidebar, its title and
//! the layout follow automatically, and so do the submenus built from `MenuRow`s (they are drawn with
//! the same `ui_row`). `Show::ClassicOnly` hides a row in Gothic, as the Font row already is.

use crate::i18n::{n_, nc_};
use crate::light::Services;
use crate::store::Settings;
use crate::ui::IconKind;

/// One category in the sidebar, top to bottom.
pub struct Category {
    pub title: &'static str,
    pub icon: IconKind,
    pub items: &'static [Item],
}

impl Category {
    /// A category: its title (a msgid), the icon by its name in the sidebar, and its rows.
    pub const fn new(title: &'static str, icon: IconKind, items: &'static [Item]) -> Category {
        Category { title, icon, items }
    }
}

/// One setting row in a category, top to bottom.
pub struct Item {
    pub icon: IconKind,
    pub label: Label,
    pub id: SettingId,
    /// A ticked/unticked line, or None for a row that opens a screen.
    pub check: Check,
    pub show: Show,
}

impl Item {
    /// A row that opens a page or a sheet, its `label` shown as it is.
    pub const fn open(icon: IconKind, label: &'static str, id: SettingId) -> Item {
        Item { icon, label: Label::Text(label), id, check: Check::None, show: Show::Always }
    }
    /// An on/off row: a switch at its right end, and `check` its state.
    pub const fn toggle(icon: IconKind, label: &'static str, id: SettingId, check: Check) -> Item {
        Item { icon, label: Label::Text(label), id, check, show: Show::Always }
    }
    /// A row whose label is built from the current settings (`Label::Style`, `Label::Font`, ...).
    pub const fn value(icon: IconKind, label: Label, id: SettingId) -> Item {
        Item { icon, label, id, check: Check::None, show: Show::Always }
    }
    /// A row that both shows a value and is an on/off choice (Mirror PDFs, with its switch).
    pub const fn value_toggle(icon: IconKind, label: Label, id: SettingId, check: Check) -> Item {
        Item { icon, label, id, check, show: Show::Always }
    }
    /// A row only the Classic style shows (Gothic has no such control).
    pub const fn classic_only(self) -> Item {
        Item { show: Show::ClassicOnly, ..self }
    }
    /// A row only where the platform leaves that service to Grimoire (`Show::Wifi`, `Show::Calibration`).
    pub const fn when(self, show: Show) -> Item {
        Item { show, ..self }
    }
}

/// Where a row's label comes from. Text is a msgid, translated as it is drawn; the other kinds
/// are built from the current settings so a row always shows its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Text(&'static str),
    /// The Display row: brightness and warmth where there is a front light, its sheet's title where not.
    Display,
    Style,
    Font,
    Language,
    SideButton,
    Eraser,
    Mirror,
    /// Darker lines in imported PDFs, with the level in use (2.2, issue #173).
    DarkerLines,
}

/// What a row does. `library::setting_hit` maps it to the hit id that `act` already handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingId {
    Display,
    Theme,
    DarkMode,
    Font,
    Rotate,
    Pinch,
    SleepSettings,
    ClockSettings,
    PenFeel,
    SideButton,
    EraserMode,
    Calibration,
    Shortcuts,
    Pair,
    Gestures,
    Language,
    Mirror,
    Wifi,
    KOReader,
    Updates,
    AboutDevice,
    DarkerLines,
    WidgetPackages,
}

/// The state a check row shows, or None for a row drawn as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    None,
    FollowRotation,
    PinchZoom,
    MirrorPdf,
    DarkMode,
}

impl Check {
    pub fn value(self, s: &Settings) -> Option<bool> {
        match self {
            Check::None => None,
            Check::FollowRotation => Some(s.follow_rotation),
            Check::PinchZoom => Some(s.pinch_zoom),
            Check::MirrorPdf => Some(s.mirror_pdf),
            Check::DarkMode => Some(s.invert),
        }
    }
}

/// Whether a row is shown: in both themes or in Classic only, and where the platform leaves its service
/// to Grimoire (`Services`, light.rs: the Kindle, the simulator and the host all of them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    Always,
    ClassicOnly,
    Wifi,
    Pairing,
    Updates,
    KOReader,
    Calibration,
}

impl Show {
    pub fn visible(self, classic: bool, services: Services) -> bool {
        match self {
            Show::Always => true,
            Show::ClassicOnly => classic,
            Show::Wifi => services.wifi,
            Show::Pairing => services.pairing,
            Show::Updates => services.updates,
            Show::KOReader => services.koreader,
            Show::Calibration => services.calibration,
        }
    }
}

/// What holding the pen's side button does while writing (settings.ini side_button 0 to 3): the row's
/// short name, then the submenu's labels.
pub static SIDE_BUTTON_NAMES: [&str; 4] = [
    // TRANSLATORS: Settings' Pen category, the row that opens the pen button's choices, with the
    // choice in use: "Pen button: Nothing".
    n_("Nothing"),
    n_("Erases"),
    n_("Highlights"),
    n_("Draws white"),
];
pub static SIDE_BUTTON_LABELS: [&str; 4] = [
    // TRANSLATORS: the pen button submenu's four choices, what holding the pen's button while writing does.
    n_("Nothing while writing"),
    n_("Erases while writing"),
    n_("Highlights while writing"),
    n_("Draws white while writing"),
];

/// What the eraser does (settings.ini eraser_mode 0 or 1): the row's short name, then the submenu's
/// labels.
pub static ERASER_NAMES: [&str; 2] = [
    // TRANSLATORS: Settings' Pen category, the row that opens the eraser's choices, with the choice in
    // use: "Eraser: Whole strokes".
    n_("Whole strokes"),
    n_("Precise"),
];
pub static ERASER_LABELS: [&str; 2] = [
    // TRANSLATORS: the eraser submenu's two choices: it takes every stroke it touches away whole, or
    // only what it touches (as white ink does).
    n_("Takes away whole strokes"),
    n_("Takes away only what it touches"),
];

/// Darker lines in imported PDFs (settings.ini pdf_gamma, by `pdfwork::Gamma`'s index): the levels, as the row shows
/// the one in use and its list offers them (2.2, issue #173). Translated with their context, `c_(DARKER_CTX, ..)`.
pub const DARKER_CTX: &str = "darker lines";
pub static DARKER_NAMES: [&str; 3] = [
    // TRANSLATORS: Settings' System category, the row that opens the darker lines choices, with the one in use
    // ("Darker lines in imported PDFs: Off"), and its list's three choices: the gray pages of a PDF imported from
    // now on are drawn as they are, a little darker, or darker still (faint lines read better on the Kindle).
    nc_("darker lines", "Off"),
    nc_("darker lines", "Medium"),
    nc_("darker lines", "Strong"),
];

/// The style row's two labels: the current theme in parentheses, as Rotation names its two
/// orientations.
pub const STYLE_GOTHIC: &str = n_("Style (Gothic)");
pub const STYLE_CLASSIC: &str = n_("Style (Classic)");

/// The Display row's label where there is a front light to name, and where there is none (the sheet's own
/// title then: no brightness or warmth).
pub const DISPLAY_WITH_LIGHT: &str = n_("Display: brightness, warmth");
pub const DISPLAY_NO_LIGHT: &str = n_("Display");

/// The Settings page, category by category and row by row. Keep this table in the order the
/// page shows; the layout never hardcodes a category or a row.
// TRANSLATORS: Settings' categories, down the left of its page, each with its icon: General's
// (language, updates, Wi-Fi), Display's (front light, theme, dark mode, font, rotation, zoom), Sleep &
// Clock's (sleep screen, timer, clock), the Pen's (tool settings, its button, the eraser, calibration),
// Controls'
// (shortcuts, pairing, gestures), System's (KOReader, USB PDF mirroring, about the device, darker lines in
// imported PDFs), and Packages' (Widgets).
pub static CATEGORIES: &[Category] = &[
    Category::new(n_("General"), IconKind::Gear, &[
        Item::value(IconKind::Language, Label::Language, SettingId::Language),
        Item::open(IconKind::Update, n_("Updates"), SettingId::Updates).when(Show::Updates),
        // TRANSLATORS: (Settings) the row that opens the Wi-Fi submenu: the radio and the Kindle's
        // saved networks.
        Item::open(IconKind::Share, n_("Wi-Fi"), SettingId::Wifi).when(Show::Wifi),
    ]),
    Category::new(n_("Display"), IconKind::Brightness, &[
        // TRANSLATORS: the row that opens Display's sheet: brightness and warmth.
        Item::value(IconKind::Brightness, Label::Display, SettingId::Display),
        Item::value(IconKind::Castle, Label::Style, SettingId::Theme),
        // TRANSLATORS: the row that turns Grimoire's dark mode (white on black) on or off.
        Item::toggle(IconKind::Invert, n_("Dark mode"), SettingId::DarkMode, Check::DarkMode),
        Item::value(IconKind::Font, Label::Font, SettingId::Font).classic_only(),
        Item::toggle(IconKind::Collapse, n_("Rotation (portrait and landscape)"), SettingId::Rotate, Check::FollowRotation),
        Item::toggle(IconKind::Search, n_("Pinch with two fingers to zoom"), SettingId::Pinch, Check::PinchZoom),
    ]),
    Category::new(n_("Sleep & Clock"), IconKind::Clock, &[
        // TRANSLATORS: the row that opens the sleep settings' page: the sleep screen and the
        // timer.
        Item::open(IconKind::Hourglass, n_("Sleep settings"), SettingId::SleepSettings),
        // TRANSLATORS: the row that opens the clock settings' page: the clock and battery on a
        // notebook page.
        Item::open(IconKind::Clock, n_("Clock settings"), SettingId::ClockSettings),
    ]),
    Category::new(n_("Pen"), IconKind::Tune, &[
        // TRANSLATORS: the row that opens Tool Settings' sheet: pressure and tilt, per tool.
        Item::open(IconKind::Tune, n_("Tool Settings: pressure and tilt per tool"), SettingId::PenFeel),
        Item::value(IconKind::Highlighter, Label::SideButton, SettingId::SideButton),
        // TRANSLATORS: the row that opens the eraser's choices: whole strokes, or precise.
        Item::value(IconKind::Eraser, Label::Eraser, SettingId::EraserMode),
        Item::open(IconKind::Target, n_("Calibrate pen and touch"), SettingId::Calibration).when(Show::Calibration),
    ]),
    Category::new(n_("Controls"), IconKind::Gamepad, &[
        Item::open(IconKind::Gamepad, n_("Shortcuts: keys and controller buttons"), SettingId::Shortcuts),
        Item::open(IconKind::Link, n_("Pair"), SettingId::Pair).when(Show::Pairing),
        Item::open(IconKind::Gestures, n_("Gestures"), SettingId::Gestures),
    ]),
    Category::new(n_("System"), IconKind::Grid, &[
        Item::open(IconKind::Notebook, n_("KOReader"), SettingId::KOReader).when(Show::KOReader),
        Item::value_toggle(IconKind::Download, Label::Mirror, SettingId::Mirror, Check::MirrorPdf),
        // TRANSLATORS: the row that opens About: what this Kindle is, its screen, and the
        // storage used.
        Item::open(IconKind::Castle, n_("About: device and storage"), SettingId::AboutDevice),
        Item::value(IconKind::Pdf, Label::DarkerLines, SettingId::DarkerLines),
    ]),
    Category::new(n_("Packages"), IconKind::Grid, &[
        // TRANSLATORS: Settings' Packages category row: the widget package manager.
        Item::open(IconKind::Download, n_("Widgets"), SettingId::WidgetPackages),
    ]),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(i: usize) -> Vec<SettingId> {
        CATEGORIES[i].items.iter().map(|it| it.id).collect()
    }

    fn text(i: usize, j: usize) -> &'static str {
        match CATEGORIES[i].items[j].label {
            Label::Text(s) => s,
            _ => "",
        }
    }

    // Adding a row is one constructor line: it brings its icon, label, action, check and theme
    // with it, and the page draws it as every other row.
    #[test]
    fn a_row_is_one_line_of_data() {
        let open = Item::open(IconKind::Gear, "Example", SettingId::Display);
        assert!(matches!(open.label, Label::Text("Example")));
        assert!(matches!(open.check, Check::None));
        assert!(matches!(open.show, Show::Always));

        let toggle = Item::toggle(IconKind::Invert, "Example", SettingId::DarkMode, Check::DarkMode);
        assert!(matches!(toggle.check, Check::DarkMode));
        assert!(matches!(toggle.show, Show::Always));

        let value = Item::value(IconKind::Castle, Label::Style, SettingId::Theme);
        assert!(matches!(value.label, Label::Style));
        assert!(matches!(value.classic_only().show, Show::ClassicOnly));

        let both = Item::value_toggle(IconKind::Download, Label::Mirror, SettingId::Mirror, Check::MirrorPdf);
        assert!(matches!(both.label, Label::Mirror));
        assert!(matches!(both.check, Check::MirrorPdf));

        let gated = Item::open(IconKind::Share, "Example", SettingId::Wifi).when(Show::Wifi);
        assert!(matches!(gated.show, Show::Wifi));
        assert!(Show::Always.visible(false, Services::ALL));
        assert!(!Show::Wifi.visible(false, Services::without("wifi")));
        assert!(Show::Wifi.visible(false, Services::ALL));
    }

    #[test]
    fn the_schema_keeps_the_settings_page_as_it_is() {
        assert_eq!(CATEGORIES.len(), 7);
        assert_eq!(
            CATEGORIES.iter().map(|c| c.title).collect::<Vec<_>>(),
            vec!["General", "Display", "Sleep & Clock", "Pen", "Controls", "System", "Packages"]
        );
        assert_eq!(ids(0), vec![SettingId::Language, SettingId::Updates, SettingId::Wifi]);
        assert_eq!(ids(1), vec![SettingId::Display, SettingId::Theme, SettingId::DarkMode, SettingId::Font, SettingId::Rotate, SettingId::Pinch]);
        assert_eq!(ids(2), vec![SettingId::SleepSettings, SettingId::ClockSettings]);
        assert_eq!(ids(3), vec![SettingId::PenFeel, SettingId::SideButton, SettingId::EraserMode, SettingId::Calibration]);
        assert_eq!(ids(4), vec![SettingId::Shortcuts, SettingId::Pair, SettingId::Gestures]);
        assert_eq!(ids(5), vec![SettingId::KOReader, SettingId::Mirror, SettingId::AboutDevice, SettingId::DarkerLines]);
        assert_eq!(ids(6), vec![SettingId::WidgetPackages]);
        assert_eq!(CATEGORIES.iter().map(|c| c.items.len()).sum::<usize>(), 23);

        assert_eq!(text(2, 0), "Sleep settings");
        assert_eq!(text(2, 1), "Clock settings");
        assert_eq!(text(0, 2), "Wi-Fi");
        assert_eq!(text(5, 0), "KOReader");
        assert_eq!(text(5, 2), "About: device and storage");
        assert!(matches!(CATEGORIES[1].items[0].label, Label::Display));
        assert!(matches!(CATEGORIES[0].items[1].show, Show::Updates));
        assert!(matches!(CATEGORIES[0].items[2].show, Show::Wifi));
        assert!(matches!(CATEGORIES[3].items[3].show, Show::Calibration));
        assert!(matches!(CATEGORIES[4].items[1].show, Show::Pairing));
        assert!(matches!(CATEGORIES[5].items[0].show, Show::KOReader));
        assert_eq!(text(1, 2), "Dark mode");
        assert_eq!(text(1, 4), "Rotation (portrait and landscape)");
        assert_eq!(text(1, 5), "Pinch with two fingers to zoom");
        assert_eq!(text(3, 0), "Tool Settings: pressure and tilt per tool");
        assert_eq!(text(3, 3), "Calibrate pen and touch");
        assert_eq!(text(4, 0), "Shortcuts: keys and controller buttons");
        assert_eq!(text(4, 1), "Pair");
        assert_eq!(text(4, 2), "Gestures");

        assert_eq!(STYLE_GOTHIC, "Style (Gothic)");
        assert_eq!(STYLE_CLASSIC, "Style (Classic)");
        assert!(matches!(CATEGORIES[1].items[1].label, Label::Style));
        assert!(matches!(CATEGORIES[1].items[2].check, Check::DarkMode));
        assert!(matches!(CATEGORIES[1].items[3].label, Label::Font));
        assert!(matches!(CATEGORIES[1].items[3].show, Show::ClassicOnly));
        assert!(matches!(CATEGORIES[1].items[4].check, Check::FollowRotation));
        assert!(matches!(CATEGORIES[1].items[5].check, Check::PinchZoom));
        assert!(matches!(CATEGORIES[3].items[1].label, Label::SideButton));
        assert!(matches!(CATEGORIES[3].items[2].label, Label::Eraser));
        assert!(matches!(CATEGORIES[0].items[0].label, Label::Language));
        assert!(matches!(CATEGORIES[5].items[1].label, Label::Mirror));
        assert!(matches!(CATEGORIES[5].items[1].check, Check::MirrorPdf));
        assert!(matches!(CATEGORIES[5].items[3].label, Label::DarkerLines));
        assert!(matches!(CATEGORIES[5].items[3].show, Show::Always));
        assert_eq!(DARKER_NAMES, ["Off", "Medium", "Strong"]);
        assert_eq!((SIDE_BUTTON_NAMES.len(), SIDE_BUTTON_LABELS.len()), (4, 4));
        assert_eq!((ERASER_NAMES.len(), ERASER_LABELS.len()), (2, 2));
    }
}
