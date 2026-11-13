# copy: tests/fixtures/fonts/good fonts
# copy: tests/fixtures/fonts/refused fonts
# generate: tests/fonts_big_fixture.py
# make l10n-check runs this in every language, in Classic, with GRIMOIRE_CHECK_FIT=1 and settings.ini's
# font=file:Gone.ttf (a font whose file is gone): Settings > Font with the owner's own fonts, every note a row
# can carry but one (tests/l10n/fonts-own-slow.qs has "Took too long to read."): "Checking..." and the notice
# "Checking 9 fonts..." while the check runs (GRIMOIRE_BG_DELAY_MS holds it back), then "Unreadable font.",
# "Too big (over 24 MB).", "No Latin letters.", "Use .ttf or .otf.", "File missing." and "Some letters come
# from Terminess.", upright and sideways (two abreast, the narrowest a row gets), on each page of them; the
# headings "Built in" and "Your fonts (grimoire/fonts)"; then a font picked (its sample drawn in it) and
# applied, the chip with its family's name, and the sheet in Terminess over it. (2.1.0-beta.2, Rust only)
wait 100
expect settings.theme is classic
expect font is terminess
tap 1535 95
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.rows is 12
waitfor notice.key is fontcheck
shot 01-checking
rotate 1
wait 100
shot 02-checking-sideways
rotate 0
wait 100
waitbg
waitfor library.font.checking is 0
waitfor notice.key is
expect library.font.state.Huge.ttf is toobig
expect library.font.state.Gone.ttf is missing
shot 03-page-1
tap library font.next
wait 50
shot 04-page-2
rotate 1
wait 100
expect library.sheet is font
shot 05-page-2-sideways
tap library font.prev
wait 50
shot 06-page-1-sideways
tap library font.file:GrimoireTest-Regular.ttf
wait 50
expect library.font.sample is own
shot 07-picked-sideways
rotate 0
wait 100
shot 08-picked
tap library font.done
wait 100
expect font is file:GrimoireTest-Regular.ttf
shot 09-settings-own
tap library font
wait 50
shot 10-sheet-own
rotate 1
wait 100
shot 11-sheet-own-sideways
rotate 0
wait 100
tap library font.terminess
wait 50
tap library font.done
wait 100
expect font is terminess
quit
