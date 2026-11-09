# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/files .
# copy: tests/fixtures/files-long .
# make l10n-check runs this in every language, Gothic and Classic, with GRIMOIRE_CHECK_FIT=1: New notebook's four
# buttons (From a file... among them), the picker's root, Documents with its folders, a long name and its pictures
# count, a PDF's card and a folder's card, and the Import place, upright and sideways. (2.1.0-beta.2, Rust only)
wait 300
tap library new
wait 200
expect library.sheet is new
shot 01-new
tap library new.file
wait 300
expect files.open is 1
shot 02-root
tap library files.item.documents
wait 600
shot 03-documents
tap library files.item.Planner.pdf
wait 300
expect files.card is pdf
shot 04-card
tap library files.card.cancel
wait 200
tap library files.more.2
wait 300
expect files.card is folder
shot 05-folder-card
tap library files.card.cancel
wait 200
rotate 1
wait 400
shot 06-documents-sideways
tap library files.item.Planner.pdf
wait 300
shot 07-card-sideways
tap library files.card.cancel
wait 200
tap library files.back
wait 300
expect files.place is root
shot 08-root-sideways
tap library files.row.1
wait 1200
shot 09-import-sideways
rotate 0
wait 400
shot 10-import
tap library files.back
wait 300
tap library files.back
wait 300
expect library.sheet is new
shot 11-new-again
rotate 1
wait 400
shot 12-new-sideways
quit
