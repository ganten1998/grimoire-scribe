# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/files .
# The picker, end to end: Library > New > From a file... opens Files with its four places, Documents lists its
# folders and PDFs (a text file is not offered), a tap on a PDF opens its card (the second tap imports), and the
# PDF becomes a notebook in the sheet's folder with the file's own name while the file stays exactly where it was.
# rust-only
wait 300
tap library new
wait 200
expect library.sheet is new
tap library new.file
wait 300
shot 01-root
expect files.open is 1
expect files.place is root
expect files.rows is 4
expect files.row.1 is Import
expect files.sub.1 is grimoire/import
expect files.row.2 is Documents
expect files.sub.2 is documents
expect files.row.3 is Downloads
expect files.row.4 is All storage
tap library files.row.2
wait 300
shot 02-documents
expect files.place is documents
expect files.crumbs is Files > Documents
expect files.rows is 3
expect files.row.1 is Books
expect files.row.2 is Scans
expect files.row.3 is Planner.pdf
expect files.card is
tap library files.row.3
wait 300
shot 03-card
expect files.card is pdf
expect files.card.rows is Import as notebook|Cancel
tap library files.card.import
wait 300
expect files.open is 0
expect library.sheet is none
waitbg
wait 300
shot 04-imported
expect library.items is 1
expect storage.exists.documents/Planner.pdf is 1
expect storage.size.documents/Planner.pdf is 1379
expect storage.names.documents is Books|Planner.pdf|Scans|notes.txt
expect storage.names.grimoire/import is .lock|.marks|README.txt|Waiting.pdf
quit
