# settings: status_mode=1
# settings: files_card=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/pdf/planner grimoire/import
# What an import holds is not renamed or moved to the trash (2.2, issue #144): while the planner in grimoire/import
# becomes a notebook (seven pages, a slice of one page every 1.5 s), its card in Files offers Rename and Delete,
# and each says "Importing now." and does nothing: no keyboard, the file where it was, nothing in the trash.
# rust-only
waitfor notice has Importing: page
tap library files-card
wait 300
tap library files.item.Import
wait 500
tap library files.item.2027_Planner.pdf
wait 300
expect files.card is pdf
expect files.card.rows has Rename|Delete|Cancel
tap library files.card.rename
wait 200
expect toast is Importing now.
expect files.keyboard is 0
expect files.card is
shot 01-busy
tap library files.item.2027_Planner.pdf
wait 300
tap library files.card.trash
wait 200
expect toast is Importing now.
# (nothing is asked of a held file: no Delete question)
expect files.card is
expect storage.names.grimoire/import has 2027 Planner.pdf
expect storage.names.grimoire/trash is
waitbg
quit
