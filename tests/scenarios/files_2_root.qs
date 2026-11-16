# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# Files' root (2.2), from the Files card: Import, the Kindle's documents and Downloads, Grimoire's Templates, Photos,
# Fonts, PDF copies and Trash, the Kindle's fonts and All storage (no "Show Files in Library": the card is there),
# with the free space line. Grimoire's PDF copies and trash are looked into here (the picker never offers them), and
# the Kindle's fonts folder lists its fonts.
# rust-only
wait 300
expect library.card.4 is files
tap library files-card
wait 300
expect files.mode is place
expect files.place is root
expect files.rows is 10
expect files.row.1 is Import
expect files.sub.1 is grimoire/import
expect files.row.2 is Documents
expect files.row.3 is Downloads
expect files.row.4 is Templates
expect files.row.5 is Photos
expect files.row.6 is Fonts
expect files.sub.6 is grimoire/fonts
expect files.row.7 is Kindle fonts
expect files.sub.7 is fonts
expect files.row.8 is PDF copies
expect files.sub.8 is grimoire/PDFs
expect files.row.9 is Trash
expect files.row.10 is All storage
expect files.free not 0
shot 01-root
tap library files.item.PDF_copies
wait 300
expect files.place is grimoire/PDFs
expect files.crumbs is Files > PDF copies
tap library files.item.School
wait 300
expect files.rows is 1
expect files.item.Gamma.pdf is Gamma.pdf
expect files.kind.1 is pdf
tap library files.back
wait 200
tap library files.back
wait 200
expect files.place is root
tap library files.item.Trash
wait 300
expect files.place is grimoire/trash
expect files.rows is 1
tap library files.back
wait 200
tap library files.item.Kindle_fonts
wait 300
expect files.crumbs is Files > Kindle fonts
expect files.item.Kindle_Serif.ttf is Kindle Serif.ttf
expect files.kind.1 is font
shot 02-kindle-fonts
tap library files.back
wait 200
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is none
quit
