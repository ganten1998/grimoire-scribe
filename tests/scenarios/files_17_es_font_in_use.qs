# settings: status_mode=1
# settings: files_card=1
# settings: theme=classic
# settings: font=file:GrimoireTest-Regular.ttf
# storage: 1
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/fonts/good grimoire/fonts
# The font in use is not deleted in Files (2.2, issue #144), in Spanish with every text checked for fit, drawn in
# that very font: its card's Delete says "Fuente en uso.", upright and sideways, and the file stays.
# rust-only
wait 100
waitbg
waitfor font is file:GrimoireTest-Regular.ttf
tap library files-card
wait 300
tap library files.item.Fonts
wait 300
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card.rows is Renombrar|Borrar|Cancelar
tap library files.card.trash
wait 300
expect toast is Fuente en uso.
expect files.card is
shot 01-in-use
rotate 1
wait 400
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.trash
wait 300
expect toast is Fuente en uso.
shot 02-in-use-sideways
expect storage.exists.grimoire/fonts/GrimoireTest-Regular.ttf is 1
quit
