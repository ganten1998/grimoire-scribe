//! assets: what the C build generates into build/gen/ with xxd -i (../Makefile), here taken with
//! include_bytes!, which embeds the files' bytes exactly as xxd does (no NUL added; the C reads
//! each with its _len). The catalogs' table comes from build.rs (po/*.mo in name order).
//!
//!   build/gen/fonts.c          FONT_* (font.rs draws them, httpd.rs serves them as /font/<name>.ttf)
//!   build/gen/eraser_sample.c  ERASER_SAMPLE (erasersample.h; pensettings.rs's practice ink)
//!   build/gen/web.c            WEB_* (httpd.rs serves the share studio and Live view inline)
//!   build/gen/l10n.c           I18N_CATALOGS (i18n.rs)

// ../Makefile's FONTS, in its order.
pub static FONT_TERMINESS_REGULAR: &[u8] = include_bytes!("../../assets/TerminessNerdFontMono-Regular.ttf");
pub static FONT_TERMINESS_BOLD: &[u8] = include_bytes!("../../assets/TerminessNerdFontMono-Bold.ttf");
pub static FONT_JACQUARD24: &[u8] = include_bytes!("../../assets/Jacquard24-Regular.ttf");
pub static FONT_JERSEY10: &[u8] = include_bytes!("../../assets/Jersey10-Regular.ttf");
pub static FONT_DOTGOTHIC16: &[u8] = include_bytes!("../../assets/DotGothic16-Regular.ttf");

/// The licenses of the fonts the share pages use, the texts the release ships in app/licenses, one
/// after another: httpd.rs serves them as /font/licenses.txt beside the fonts (Rust only).
/// Terminess's icons are Material Design Icons under the Apache License 2.0, whose text must go
/// with every copy; Terminus, Jacquard 24 and Jersey 10 are under the SIL Open Font License 1.1.
pub static FONT_LICENSES: &str = concat!(
    "The licenses of the fonts Grimoire's share pages use (/font/terminess.ttf, terminess-bold.ttf,\n",
    "jacquard24.ttf and jersey10.ttf): Terminess, a subset of Nerd Fonts' Terminess Nerd Font Mono\n",
    "(Terminus), under the SIL Open Font License 1.1, with Material Design Icons inside it under the\n",
    "Apache License 2.0; Jacquard 24 and Jersey 10 under the SIL Open Font License 1.1.\n",
    "\n\n---------- Terminess-OFL-1.1.txt ----------\n\n",
    include_str!("../../assets/Terminess-OFL-1.1.txt"),
    "\n\n\n---------- MaterialDesignIcons-LICENSE.txt ----------\n\n",
    include_str!("../../assets/MaterialDesignIcons-LICENSE.txt"),
    "\n\n---------- MaterialDesignIcons-Apache-2.0.txt ----------\n",
    include_str!("../../assets/MaterialDesignIcons-Apache-2.0.txt"),
    "\n\n---------- Jacquard24-OFL-1.1.txt ----------\n\n",
    include_str!("../../assets/Jacquard24-OFL-1.1.txt"),
    "\n\n---------- Jersey10-OFL-1.1.txt ----------\n\n",
    include_str!("../../assets/Jersey10-OFL-1.1.txt"),
);

/// The maintainer's handwritten eraser practice, kept as strokes rather than a flat picture
/// (assets/eraser-sample.qpg; the C's grimoire_eraser_sample).
pub static ERASER_SAMPLE: &[u8] = include_bytes!("../../assets/eraser-sample.qpg");

// ../Makefile's WEB, in its order: the share studio's page and Live view's, served inline by httpd.c.
pub static WEB_STUDIO_JS: &[u8] = include_bytes!("../../web/studio.js");
pub static WEB_STUDIO_CSS: &[u8] = include_bytes!("../../web/studio.css");
pub static WEB_DOWNLOAD_JS: &[u8] = include_bytes!("../../web/download.js");
pub static WEB_LIVE_JS: &[u8] = include_bytes!("../../web/live.js");
pub static WEB_LIVE_CSS: &[u8] = include_bytes!("../../web/live.css");

// The catalogs of po/*.mo (build.rs writes the table).
include!(concat!(env!("OUT_DIR"), "/l10n.rs"));

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_as_the_files() {
        let top = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        assert_eq!(super::FONT_JERSEY10, &std::fs::read(format!("{top}/assets/Jersey10-Regular.ttf")).unwrap()[..]);
        assert_eq!(super::WEB_LIVE_JS, &std::fs::read(format!("{top}/web/live.js")).unwrap()[..]);
        assert!(super::I18N_CATALOGS.iter().any(|c| c.code == "es"));
    }

    #[test]
    fn the_font_licenses_are_the_shipped_texts() {
        let top = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
        let mut at = 0;
        for name in ["Terminess-OFL-1.1.txt", "MaterialDesignIcons-LICENSE.txt", "MaterialDesignIcons-Apache-2.0.txt",
                     "Jacquard24-OFL-1.1.txt", "Jersey10-OFL-1.1.txt"] {
            let text = std::fs::read_to_string(format!("{top}/assets/{name}")).unwrap();
            let found = super::FONT_LICENSES[at..].find(&text).unwrap_or_else(|| panic!("{name} is not in FONT_LICENSES"));
            at += found + text.len();
        }
    }
}
