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
}
