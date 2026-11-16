# settings: status_mode=1
# settings: shortcuts_enabled=1
# storage: 1
# generate: tests/files_place_tree.py
# copy: tests/fixtures/files-keys grimoire
# Files' two actions (2.2), bound by hand in shortcuts.ini (neither has a default key): files opens Files whether or
# not its card is in the Library (here it is not), and inside Files goes back to its root; home leaves Files, and
# any Library folder, for the Library's top.
# rust-only
wait 300
expect library.items is 3
key 33 0
wait 300
expect files.open is 1
expect files.mode is place
expect files.place is root
expect files.rows is 11
expect files.row.11 is Show Files in Library
tap library files.item.documents
wait 300
expect files.place is documents
key 33 0
wait 300
expect files.place is root
tap library files.item.Fonts
wait 300
key 35 0
wait 300
expect files.open is 0
expect library.sheet is none
tap library folder.School
wait 300
expect library.items is 1
key 35 0
wait 300
expect library.items is 3
expect library.card.1 is folder
quit
