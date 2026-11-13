# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/files .
# The picker's imports: a name typed in New notebook's sheet is the notebook's, else the file's own name; a PDF
# the Library already holds shows "has notes" and offers Open its notebook; a folder of pictures is imported
# from its `...` card, in place (its files stay as they were). (Keyboard, upright: del 1679 2205, q 133 1885,
# w 310 1885, Done 1685 2365.)
# rust-only
wait 300
tap library new
wait 200
tap library new.rename
wait 200
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 133 1885
tap 310 1885
tap 1685 2365
wait 200
expect library.sheet is new
tap library new.file
wait 300
tap library files.item.documents
wait 300
tap library files.item.Planner.pdf
wait 200
tap library files.card.import
wait 300
waitbg
wait 300
expect library.items is 1
expect library.card.1.name is qw
tap library new
wait 200
tap library new.file
wait 300
tap library files.item.Downloads
wait 300
tap library files.item.Daily.pdf
wait 200
tap library files.card.import
wait 300
waitbg
wait 300
shot 01-two
expect library.items is 2
expect library.card.1.name is Daily
expect library.card.2.name is qw
tap library new
wait 200
tap library new.file
wait 300
tap library files.item.documents
wait 300
expect files.item.Scans is Scans
tap library files.more.2
wait 300
shot 02-folder-card
expect files.card is folder
expect files.card.details is Scans
expect files.card.rows is Open|Import 3 pictures as notebook|Cancel
tap library files.card.pictures
wait 300
waitbg
wait 300
expect library.items is 3
expect library.card.1.name is Scans
expect storage.names.documents/Scans is notes.txt|page-1.png|page-10.png|page-2.jpg
expect storage.names.documents is Books|Planner.pdf|Scans|notes.txt
tap library new
wait 200
tap library new.file
wait 300
tap library files.item.documents
wait 500
shot 03-has-notes
expect files.notes.3 is 1
tap library files.item.Planner.pdf
wait 200
expect files.card.rows is Open its notebook|Cancel
tap library files.card.notebook
wait 600
expect screen is editor
quit
