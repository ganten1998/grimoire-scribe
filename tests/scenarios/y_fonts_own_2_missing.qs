# settings: status_mode=1
# settings: theme=classic
# settings: font=file:Gone.ttf
# settings: font_bold=file:Gone-Bold.ttf
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/fonts/many fonts
# The font settings.ini names is no longer in grimoire/fonts (deleted over USB): Grimoire draws in Terminess,
# the setting stays as it is (the file put back brings the font back), and the Font sheet lists it greyed,
# "File missing.". Twelve fonts and that row take two pages, upright (eight rows a page on the new Settings
# page, #115) and sideways, where the rows stand two abreast (twelve a page); turned, the sheet shows the
# page that holds the first row it showed. Another setting saved keeps the lines; a pick and Done (the
# sheet's X) replaces them. (2.1.0-beta.2)
# rust-only
wait 100
expect font is terminess
expect settings.font is file:Gone.ttf
expect settings.font_bold is Gone-Bold.ttf
tap 1535 95
wait 50
tap library settings.cat.1
wait 100
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
expect library.font.rows is 13
expect library.font.state.Gone.ttf is missing
expect library.font.note.Gone.ttf is File missing.
waitbg
waitfor library.font.checking is 0
expect library.font.page is 1/2
shot 01-first-page
tap library font.next
wait 50
expect library.font.shown is Test09.ttf|Test10.ttf|Test11.ttf|Test12.ttf|Gone.ttf
expect library.ui_has.font.file:Gone.ttf is 0
shot 02-missing
rotate 1
wait 100
expect library.sheet is font
expect library.font.page is 1/2
tap library font.next
wait 50
expect library.font.page is 2/2
expect library.font.shown is Gone.ttf
shot 03-sideways
rotate 0
wait 100
tap library font.cancel
wait 50
# The sheet's X on Terminess, which draws now: nothing changes, the lines stay (the X closes Settings)
tap library font
wait 50
tap library font.done
wait 50
expect library.sheet is none
expect settings.font is file:Gone.ttf
expect settings.font_bold is Gone-Bold.ttf
# Another setting saved: the lines stay
tap 1535 95
wait 50
tap library settings.cat.6
wait 100
tap library mirror
wait 50
tap library mirror
wait 50
expect settings.font is file:Gone.ttf
expect settings.font_bold is Gone-Bold.ttf
# A pick and Done: the font, and the Bold file named for the other one is forgotten
tap library settings.cat.1
wait 100
tap library font
wait 50
tap library font.file:Test03.ttf
wait 50
tap library font.done
wait 100
expect font is file:Test03.ttf
expect settings.font is file:Test03.ttf
expect settings.font_bold is
shot 04-settings-test3
quit
