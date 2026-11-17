# settings: status_mode=1
# settings: theme=classic
# settings: files_card=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_BG_DELAY_MS=2500
# copy: tests/fixtures/fonts/good fonts
# Add to fonts on a font's card in Files (2.2, issue #161), then Settings > Font opened in the Library while the copy
# still runs. The sheet waits for that copy as for one its own Add a font... asked (the row an hourglass, no tap),
# then reads the folder again and lists the new file, being checked and then ready, with no toast: the user does not
# have to leave and open the sheet again to see it. GRIMOIRE_BG_DELAY_MS holds the copy back so the sheet opens
# while it runs.
# rust-only
wait 100
tap library files-card
wait 300
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.add
wait 200
expect bg.busy is 1
tap library files.back
wait 200
tap library files.back
wait 300
expect files.open is 0
tap 1535 95
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.copying is 1
expect library.ui_has.font.add is 0
expect library.font.rows is 0
waitfor library.font.copying is 0
expect toast is
expect library.font.rows is 1
expect library.ui_has.font.add is 1
expect library.font.state.GrimoireTest-Regular.ttf is checking
waitfor library.font.checking is 0
expect library.font.state.GrimoireTest-Regular.ttf is ready
expect font is terminess
tap library font.done
wait 100
expect library.sheet is none
quit
