# settings: status_mode=1
# settings: theme=classic
# settings: files_card=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_BG_DELAY_MS=2000
# copy: tests/fixtures/fonts/good fonts
# Add a font... twice in a row from Settings > Font (2.2, issue #161), the second pick opened while the first font's
# check runs. The sheet's notice "Checking 1 font..." is the sheet's: it is not up over the font pick, and the pick's
# root and folders show none. The second copy asked, the sheet is back and waits for it; the second font is then
# checked by itself too, and both are ready without the sheet being opened again. GRIMOIRE_BG_DELAY_MS holds each
# job back so the pick opens while the check runs.
# rust-only
wait 100
tap library settings
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
tap library font.add
wait 300
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.add
wait 300
waitfor library.font.copying is 0
expect library.font.state.GrimoireTest-Regular.ttf is checking
waitfor notice is Checking 1 font...
waitfor bg.busy is 1
# the pick over the sheet while that check runs: no notice over it
tap library font.add
wait 300
expect files.open is 1
expect files.place is root
expect notice is
expect library.font.state.GrimoireTest-Regular.ttf is checking
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Bold.ttf
wait 300
expect files.card.rows is Add to fonts|Cancel
expect notice is
tap library files.card.add
wait 300
expect library.sheet is font
expect library.font.copying is 1
waitfor library.font.copying is 0
expect library.font.rows is 2
waitfor library.font.checking is 0
waitfor notice is
expect library.font.state.GrimoireTest-Regular.ttf is ready
expect library.font.state.GrimoireTest-Bold.ttf is ready
expect font is terminess
quit
