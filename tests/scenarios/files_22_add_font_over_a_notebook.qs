# settings: status_mode=1
# settings: theme=classic
# settings: files_card=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_BG_DELAY_MS=2500
# copy: tests/fixtures/fonts/good fonts
# Add to fonts (2.2, issue #161), then a notebook opened and Settings > Font opened over its page while the copy
# still runs. The sheet there has no Add a font... row; it waits for the copy as in the Library, then reads the folder
# again and lists the new file, being checked and then ready. That sheet shows the copy's end itself: no "Copied to
# Fonts." toast over it. GRIMOIRE_BG_DELAY_MS holds the copy back so the sheet opens while it runs.
# rust-only
wait 100
tap library files-card
wait 300
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.add
wait 100
expect bg.busy is 1
tap library files.back
wait 100
tap library files.back
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
tap panel settings
wait 300
tap library settings.cat.1
wait 100
tap library font
wait 100
expect library.sheet is font
expect library.ui_has.font.add is 0
expect library.font.copying is 1
expect library.font.rows is 0
waitfor library.font.copying is 0
expect toast is
expect library.font.rows is 1
expect library.font.state.GrimoireTest-Regular.ttf is checking
waitfor library.font.checking is 0
expect library.font.state.GrimoireTest-Regular.ttf is ready
expect toast is
expect font is terminess
tap library font.done
wait 300
expect editor.modal is none
quit
