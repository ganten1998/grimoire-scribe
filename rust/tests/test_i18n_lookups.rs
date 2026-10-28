// test_i18n_lookups: i18n.rs's own lookup checks, kept out of rust/src. make l10n reads every
// tr(), c_() and ng_() in rust/src/*.rs into po/grimoire.pot, and these look up msgids no screen
// shows (one in no catalog, a context no string has, an empty one): there they would become entries
// for translators. (test_i18n.rs is test_i18n.c's port; this file has no C twin.)

use grimoire::i18n::*;

// (the language is thread state: the test names the one it wants and ends in English)
fn english() {
    i18n_set("en");
}

#[test]
fn lookups_contexts_and_plurals() {
    assert!(i18n_set("es") && i18n_current() == "es");
    assert_eq!(tr("Done"), "Listo");
    assert_eq!(tr("Cancel"), "Cancelar");
    assert_eq!(tr("This is not in any catalog"), "This is not in any catalog");
    assert_eq!(tr(""), "");
    // A context tells two meanings apart: "key" is not the plain lookup.
    assert_eq!(c_("key", "space"), "espacio");
    assert_eq!(c_("nope", "Done"), "Done");
    assert_eq!(c_("paper", "Blank"), "En blanco");
    // The plural form of a count, in the language's rule.
    assert_eq!(ng_("%d page", "%d pages", 1), "%d p\u{e1}gina");
    assert_eq!(ng_("%d page", "%d pages", 0), "%d p\u{e1}ginas");
    assert_eq!(ng_("%d page", "%d pages", 2), "%d p\u{e1}ginas");
    assert_eq!(ng_("%d thing", "%d things", 1), "%d thing");
    assert_eq!(ng_("%d thing", "%d things", 3), "%d things");
    assert_eq!(i18n_getn_many("%d page", "%d pages", 1), ng_("%d page", "%d pages", 1));
    english();
    assert_eq!(tr("Done"), "Done");
    assert_eq!(ng_("%d page", "%d pages", 1), "%d page");
    assert_eq!(ng_("%d page", "%d pages", 2), "%d pages");
}
