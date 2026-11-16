# root: files_15_font_in_use
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# The font renamed in Files by files_15_font_in_use, and its Bold file, are the font when Grimoire starts again
# (2.2, issue #144): settings.ini names the new paths, so the start does not look for the old names and stay in
# Terminess. (This run's data root is a copy of that run's, whose files have the time of the copy: every font is
# checked once more here, and the font comes back when its check ends. The start that needs no check is
# test_sec_font_verdict.rs's, on the data root the rename was made in.)
# rust-only
wait 100
expect settings.theme is classic
expect settings.font is file:GrimoireTest-Regular2.ttf
expect settings.font_bold is GrimoireTest-Bold2.ttf
waitbg
waitfor font is file:GrimoireTest-Regular2.ttf
expect font.own.drawn is 1
shot 01-library
tap 1535 95
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.font.pick is file:GrimoireTest-Regular2.ttf
waitfor library.font.state.GrimoireTest-Regular2.ttf is ready
expect library.font.state.GrimoireTest-Regular.ttf is
quit
