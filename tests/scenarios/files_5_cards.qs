# settings: status_mode=1
# settings: files_card=1
# settings: files_show=all
# storage: 1
# generate: tests/files_place_tree.py
# Files' item cards (2.2): a tap on a picture, a font or another file (or its "...") opens its card, which says what
# the file is with its size and date and offers nothing yet but Cancel (Use as template, Use as photo and Add to fonts
# come with the copy job); a PDF's card imports it as the picker's does; a PDF of the PDF copies opens the notebook it
# was written from; a link has no card; the trash lists its entries read only, a notebook by its own name.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Sorted
wait 300
expect files.show is all
tap library files.item.c.png
wait 300
expect files.card is picture
expect files.card.details has Picture, 2 KB,
expect files.card.rows is Cancel
shot 01-picture
tap library files.card.cancel
wait 200
expect files.card is
tap library files.item.d.ttf
wait 300
expect files.card is font
expect files.card.details has Font, 1 KB,
expect files.card.rows is Cancel
shot 02-font
tap library files.card.cancel
wait 200
expect files.row.6 is e.txt
tap library files.more.6
wait 300
expect files.card is other
expect files.card.details has File, 4 KB,
expect files.card.rows is Cancel
shot 03-other
tap library files.card.cancel
wait 200
tap library files.item.f.pdf
wait 200
expect files.card is
tap library files.item.a.pdf
wait 300
expect files.card is pdf
expect files.card.details has PDF, 1 KB,
expect files.card.rows is Import as notebook|Cancel
tap library files.card.cancel
wait 200
tap library files.crumb.1
wait 300
tap library files.item.Trash
wait 400
expect files.rows is 1
expect files.row.1 is Gone
expect files.kind.1 is trashed
tap library files.row.1
wait 200
expect files.card is
expect files.place is grimoire/trash
shot 04-trash
tap library files.crumb.1
wait 300
tap library files.item.PDF_copies
wait 300
tap library files.item.School
wait 300
tap library files.item.Gamma.pdf
wait 300
expect files.card is pdf
expect files.card.rows is Open its notebook|Cancel
shot 05-mirror
tap library files.card.notebook
wait 600
expect files.open is 0
expect screen is editor
quit
