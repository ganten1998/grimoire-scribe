# copy: tests/fixtures/fonts/good fonts
# make l10n-check runs this in every language, in Classic, with GRIMOIRE_CHECK_FIT=1 and the font worker's hang
# hook (GRIMOIRE_FONT_TEST_HANG, with a short GRIMOIRE_FONT_WATCHDOG_MS): every font of the folder takes longer
# than a font may, and its row says "Took too long to read.", upright and sideways. (2.1.0-beta.2, Rust only)
wait 100
tap 1535 95
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.rows is 4
waitbg
waitfor library.font.checking is 0
expect library.font.state.GrimoireTest-Regular.ttf is slow
shot 01-slow
rotate 1
wait 100
shot 02-slow-sideways
rotate 0
wait 100
tap library font.cancel
wait 50
quit
