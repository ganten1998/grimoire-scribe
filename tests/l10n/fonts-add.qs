# settings: status_mode=1
# settings: files_card=1
# storage: 1
# copy: tests/fixtures/fonts/good fonts
# make l10n-check runs this in every language, in Classic (in Terminess, and in each other font in English and
# Spanish), with GRIMOIRE_CHECK_FIT=1 and files_card=1: Add a font... (2.2, issue #161). Settings > Font with no
# fonts of your own (the row under the folder's text), upright and sideways; the font pick it opens (its title, the
# Kindle's own fonts folder first), that folder's fonts and a font's card (Add to fonts), upright and sideways; the
# sheet again with the row waiting for the copy (GRIMOIRE_BG_DELAY_MS holds each job back), then the new font being
# checked, then checked; last, a font's card in Files (its details and Add to fonts), upright and sideways. (Rust
# only)
# rust-only
wait 300
expect settings.theme is classic
tap library settings
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.ui_has.font.add is 1
shot 01-sheet
rotate 1
wait 100
expect library.ui_has.font.add is 1
shot 02-sheet-sideways
rotate 0
wait 100
tap library font.add
wait 300
expect files.open is 1
expect files.mode is pick
shot 03-pick-root
tap library files.item.Kindle_fonts
wait 300
expect files.place is fonts
shot 04-kindle-fonts
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
shot 05-card
tap library files.card.cancel
wait 200
rotate 1
wait 400
shot 06-kindle-fonts-sideways
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
shot 07-card-sideways
tap library files.card.cancel
wait 200
rotate 0
wait 400
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.add
wait 300
expect library.sheet is font
expect library.font.copying is 1
shot 08-copying
waitfor library.font.copying is 0
expect library.font.state.GrimoireTest-Regular.ttf is checking
shot 09-checking
waitfor library.font.checking is 0
expect library.font.state.GrimoireTest-Regular.ttf is ready
shot 10-checked
tap library font.done
wait 100
expect library.sheet is none
tap library files-card
wait 300
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Bold.ttf
wait 300
expect files.card is font
shot 11-files-card
tap library files.card.cancel
wait 200
rotate 1
wait 400
tap library files.item.GrimoireTest-Bold.ttf
wait 300
expect files.card is font
shot 12-files-card-sideways
quit
