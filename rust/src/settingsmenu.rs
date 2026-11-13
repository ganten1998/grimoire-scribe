//! Settings' categories and rows as data: the backend of the Settings page.
//!
//! `library::draw_settings` is layout only. To add a row, add an `Item` here and handle its
//! `SettingId` in `library::setting_hit` (plus `library::act` if it needs a new action). To add
//! a category, add a `Category` here: the sidebar, its title and the layout follow automatically.
//! `Show::ClassicOnly` hides a row in Gothic, as the Font row already is.

use crate::i18n::n_;
use crate::store::Settings;
use crate::ui::IconKind;

/// One category in the sidebar, top to bottom.
pub struct Category {
    pub title: &'static str,
    pub icon: IconKind,
    pub items: &'static [Item],
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

/// Where a row's label comes from. Text is a msgid, translated as it is drawn; the other kinds
/// are built from the current settings so a row always shows its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Text(&'static str),
    Style,
    Font,
    Language,
    SideButton,
    Mirror,
}

/// What a row does. `library::setting_hit` maps it to the hit id that `act` already handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingId {
    Display,
    Theme,
    Font,
    Rotate,
    Pinch,
    PenFeel,
    SideButton,
    Calibration,
    Shortcuts,
    Pair,
    Gestures,
    Language,
    Mirror,
    Wifi,
    KOReader,
    Updates,
}

/// The state a check row shows, or None for a row drawn as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    None,
    FollowRotation,
    PinchZoom,
    MirrorPdf,
}

impl Check {
    pub fn value(self, s: &Settings) -> Option<bool> {
        match self {
            Check::None => None,
            Check::FollowRotation => Some(s.follow_rotation),
            Check::PinchZoom => Some(s.pinch_zoom),
            Check::MirrorPdf => Some(s.mirror_pdf),
        }
    }
}

/// Whether a row is shown in both themes, or in Classic only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    Always,
    ClassicOnly,
}

impl Show {
    pub fn visible(self, classic: bool) -> bool {
        match self {
            Show::Always => true,
            Show::ClassicOnly => classic,
        }
    }
}

/// What holding the pen's side button does while writing (settings.ini side_button 0 to 3).
pub static SIDE_BUTTON_LABELS: [&str; 4] = [
    // TRANSLATORS: what holding the pen's button while writing does; a tap moves to the next.
    n_("Holding the pen button while writing does nothing (tap to change)"),
    n_("Holding the pen button while writing erases (tap to change)"),
    n_("Holding the pen button while writing highlights (tap to change)"),
    n_("Holding the pen button while writing draws white (tap to change)"),
];

/// The style row's two labels: the current theme, and the one a tap switches to.
pub const STYLE_GOTHIC: &str = n_("Style: Gothic (tap for Classic)");
pub const STYLE_CLASSIC: &str = n_("Style: Classic (tap for Gothic)");

/// The Settings page, category by category and row by row. Keep this table in the order the
/// page shows; the layout never hardcodes a category or a row.
// TRANSLATORS: Settings' categories, down the left of its page, each with its icon: the settings
// they group are General's (language, mirroring, Wi-Fi, KOReader, updates), Display's, Pen Feel's,
// Calibration's and the Controller's (shortcuts, pairing, gestures).
pub static CATEGORIES: &[Category] = &[
    Category {
        title: n_("General"),
        icon: IconKind::Gear,
        items: &[
            Item {
                icon: IconKind::Language,
                label: Label::Language,
                id: SettingId::Language,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Download,
                label: Label::Mirror,
                id: SettingId::Mirror,
                check: Check::MirrorPdf,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Share,
                // TRANSLATORS: (Settings) the row that opens the Wi-Fi submenu: the radio and the Kindle's
                // saved networks.
                label: Label::Text(n_("Wi-Fi")),
                id: SettingId::Wifi,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Notebook,
                label: Label::Text(n_("KOReader")),
                id: SettingId::KOReader,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Update,
                label: Label::Text(n_("Updates")),
                id: SettingId::Updates,
                check: Check::None,
                show: Show::Always,
            },
        ],
    },
    Category {
        title: n_("Display"),
        icon: IconKind::Brightness,
        items: &[
            Item {
                icon: IconKind::Brightness,
                // TRANSLATORS: the row that opens Display's sheet: brightness, warmth and invert.
                label: Label::Text(n_("Display: brightness, warmth, invert")),
                id: SettingId::Display,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Castle,
                label: Label::Style,
                id: SettingId::Theme,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Font,
                label: Label::Font,
                id: SettingId::Font,
                check: Check::None,
                show: Show::ClassicOnly,
            },
            Item {
                icon: IconKind::Collapse,
                label: Label::Text(n_("Turn with the Kindle (portrait and landscape)")),
                id: SettingId::Rotate,
                check: Check::FollowRotation,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Search,
                label: Label::Text(n_("Pinch with two fingers to zoom")),
                id: SettingId::Pinch,
                check: Check::PinchZoom,
                show: Show::Always,
            },
        ],
    },
    Category {
        title: n_("Pen Feel"),
        icon: IconKind::Tune,
        items: &[
            Item {
                icon: IconKind::Tune,
                // TRANSLATORS: the row that opens Pen sensitivity's sheet: pressure and tilt, per tool.
                label: Label::Text(n_("Pen sensitivity (pressure + tilt per tool)")),
                id: SettingId::PenFeel,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Highlighter,
                label: Label::SideButton,
                id: SettingId::SideButton,
                check: Check::None,
                show: Show::Always,
            },
        ],
    },
    Category {
        title: n_("Calibration"),
        icon: IconKind::Target,
        items: &[Item {
            icon: IconKind::Target,
            label: Label::Text(n_("Calibrate pen and touch")),
            id: SettingId::Calibration,
            check: Check::None,
            show: Show::Always,
        }],
    },
    Category {
        title: n_("Controller"),
        icon: IconKind::Gamepad,
        items: &[
            Item {
                icon: IconKind::Gamepad,
                label: Label::Text(n_("Shortcuts: keys and controller buttons")),
                id: SettingId::Shortcuts,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Link,
                label: Label::Text(n_("Pair")),
                id: SettingId::Pair,
                check: Check::None,
                show: Show::Always,
            },
            Item {
                icon: IconKind::Gestures,
                label: Label::Text(n_("Gestures")),
                id: SettingId::Gestures,
                check: Check::None,
                show: Show::Always,
            },
        ],
    },
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

    #[test]
    fn the_schema_keeps_the_settings_page_as_it_is() {
        assert_eq!(CATEGORIES.len(), 5);
        assert_eq!(
            CATEGORIES.iter().map(|c| c.title).collect::<Vec<_>>(),
            vec!["General", "Display", "Pen Feel", "Calibration", "Controller"]
        );
        assert_eq!(ids(0), vec![SettingId::Language, SettingId::Mirror, SettingId::Wifi, SettingId::KOReader, SettingId::Updates]);
        assert_eq!(ids(1), vec![SettingId::Display, SettingId::Theme, SettingId::Font, SettingId::Rotate, SettingId::Pinch]);
        assert_eq!(ids(2), vec![SettingId::PenFeel, SettingId::SideButton]);
        assert_eq!(ids(3), vec![SettingId::Calibration]);
        assert_eq!(ids(4), vec![SettingId::Shortcuts, SettingId::Pair, SettingId::Gestures]);
        assert_eq!(CATEGORIES.iter().map(|c| c.items.len()).sum::<usize>(), 16);

        assert_eq!(text(0, 2), "Wi-Fi");
        assert_eq!(text(0, 3), "KOReader");
        assert_eq!(text(0, 4), "Updates");
        assert_eq!(text(1, 0), "Display: brightness, warmth, invert");
        assert_eq!(text(1, 3), "Turn with the Kindle (portrait and landscape)");
        assert_eq!(text(1, 4), "Pinch with two fingers to zoom");
        assert_eq!(text(2, 0), "Pen sensitivity (pressure + tilt per tool)");
        assert_eq!(text(3, 0), "Calibrate pen and touch");
        assert_eq!(text(4, 0), "Shortcuts: keys and controller buttons");
        assert_eq!(text(4, 1), "Pair");
        assert_eq!(text(4, 2), "Gestures");

        assert!(matches!(CATEGORIES[1].items[1].label, Label::Style));
        assert!(matches!(CATEGORIES[1].items[2].label, Label::Font));
        assert!(matches!(CATEGORIES[1].items[2].show, Show::ClassicOnly));
        assert!(matches!(CATEGORIES[1].items[3].check, Check::FollowRotation));
        assert!(matches!(CATEGORIES[1].items[4].check, Check::PinchZoom));
        assert!(matches!(CATEGORIES[2].items[1].label, Label::SideButton));
        assert!(matches!(CATEGORIES[0].items[0].label, Label::Language));
        assert!(matches!(CATEGORIES[0].items[1].label, Label::Mirror));
        assert!(matches!(CATEGORIES[0].items[1].check, Check::MirrorPdf));
        assert_eq!(SIDE_BUTTON_LABELS.len(), 4);
    }
}
