# settings: status_mode=1
# settings: theme=classic
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_FONT_TEST_HANG=40
# env: GRIMOIRE_FONT_WATCHDOG_MS=400
# copy: tests/fixtures/fonts/good fonts
# generate: tests/fonts_big_fixture.py
# The two refusals y_fonts_own_1_sheet does not reach: a font file over 24 MB (30 MB here, sparse: it is refused
# by its size, never read) says "Too big (over 24 MB)."; and a font whose check does not end (the worker's hang
# hook, with a short watchdog) says "Took too long to read.". Neither takes a tap, neither is tried again when
# the sheet opens again, and Grimoire stays in Terminess. (2.1.0-beta.2)
# rust-only
wait 100
tap 1535 95
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.rows is 5
expect library.font.state.Huge.ttf is toobig
expect library.font.note.Huge.ttf is Too big (over 24 MB).
waitbg
waitfor library.font.checking is 0
expect library.font.state.GrimoireTest-Regular.ttf is slow
expect library.font.note.GrimoireTest-Regular.ttf is Took too long to read.
expect library.font.state.GrimoireTestCFF.otf is slow
expect library.ui_has.font.file:GrimoireTest-Regular.ttf is 0
expect library.ui_has.font.file:Huge.ttf is 0
waitfor notice is
shot 01-too-big-and-too-slow
tap library font.cancel
wait 50
# Opened again: their verdicts stand, nothing is checked again
tap library font
wait 50
expect library.font.checking is 0
expect library.font.state.GrimoireTest-Bold.ttf is slow
expect bg.children is 0
tap library font.cancel
wait 50
expect font is terminess
quit
