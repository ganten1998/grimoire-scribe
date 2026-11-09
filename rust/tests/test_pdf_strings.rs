// test_pdf_strings.rs: the PDF import's words fit a toast in every language and both styles (2.1.0-beta.2): the
// widest ones ("Importing: page 2000 of 2000", "PDF too big: over 512 MB.", the unreadable-pages ending) wrap to
// at most the two lines a toast and the staying notice have room for.
use grimoire::prelude::*;

#[test]
fn the_widest_import_texts_fit_two_lines_in_every_language_and_style() {
    ui_init(300.0);
    assert_eq!(font_init(), 0);
    let screen = Rect { x: 0, y: 0, w: 1860, h: 2480 };
    for style in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(style);
        for i in 0..i18n_language_count() {
            let code = i18n_language_code(i).unwrap();
            assert!(i18n_set(code));
            let texts = [
                cformat!(tr("Importing: page %d of %d"), 2000, 2000),
                cformat!(tr("PDF too big: over %d MB."), 512),
                cformat!(tr("PDF too long: notebooks hold %d pages."), 2000),
                cformat!(tr("Couldn't import PDF (see %s)."), "grimoire.log"),
                tr("Couldn't import PDF. Is the storage full?").to_string(),
                tr("PDF needs a password.").to_string(),
                tr("PDF already imported.").to_string(),
                cformat!(ng_("Imported notebook (%d page). Some pages couldn't be read.", "Imported notebook (%d pages). Some pages couldn't be read.", 2000), 2000),
            ];
            for t in &texts {
                let lines = font_wrap_lines(FontFace::Bold, ui_text_size(), t, ui_toast_room(screen), None);
                assert!(lines <= 2, "{code}: {t:?} takes {lines} lines");
                let r = ui_toast_rect(screen, t);
                assert!(r.w <= screen.w && r.x >= 0, "{code}: {t:?}");
            }
        }
    }
    assert!(i18n_set("en"));
}
