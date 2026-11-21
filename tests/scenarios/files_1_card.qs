# settings: status_mode=1
# storage: 1
# generate: tests/files_place_tree.py
# The Files card (2.2): not in the Library at first. The file picker's last root row, "Show Files in Library",
# puts it at the end of the Library, after every folder and notebook of the top level (and in no folder); a tap on
# it opens Files at its root, which has no such row now; its "..." opens its menu, whose Hide from Library takes it
# away again, and the picker's row comes back.
# rust-only
wait 300
expect library.items is 3
expect library.card.3 not files
tap library add
tap library add.new-notebook
wait 200
tap library new.file
wait 300
expect files.mode is pick
expect files.rows is 5
expect files.row.5 is Show Files in Library
expect files.kind.5 is show
shot 01-picker-root
tap library files.row.5
wait 300
expect files.rows is 4
expect files.row.4 is All storage
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is new
tap library done
wait 300
expect library.sheet is none
expect library.items is 4
expect library.card.1 is folder
expect library.card.4 is files
expect library.card.4.name is Files
shot 02-card-last
tap library folder.School
wait 300
expect library.items is 1
expect library.card.1 not files
tap library back
wait 300
tap library files-card
wait 300
expect files.open is 1
expect files.mode is place
expect files.place is root
expect files.item.Show_Files_in_Library is
expect files.item.All_storage is All storage
shot 03-files-root
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is none
tap library files-card-menu
wait 300
expect library.sheet is files-more
shot 04-card-menu
tap library files-card.hide
wait 300
expect library.sheet is none
expect library.items is 3
expect library.card.3 not files
tap library add
tap library add.new-notebook
wait 200
tap library new.file
wait 300
expect files.row.5 is Show Files in Library
quit
