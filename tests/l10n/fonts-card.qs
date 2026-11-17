# settings: status_mode=1
# settings: files_card=1
# storage: 1
# copy: tests/fixtures/fonts/good fonts
# make l10n-check runs this in every language, in Gothic, with GRIMOIRE_CHECK_FIT=1 and files_card=1: a font's card
# in Files (2.2, issue #161), with its details and Add to fonts, in the Kindle's own fonts folder, upright and
# sideways. (Classic draws the same card in tests/l10n/fonts-add.qs, after the Font sheet, which is Classic's.)
# (Rust only)
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.Kindle_fonts
wait 300
expect files.place is fonts
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
shot 01-card
tap library files.card.cancel
wait 200
rotate 1
wait 400
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
shot 02-card-sideways
quit
