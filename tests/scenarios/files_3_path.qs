# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# Files' path bar (2.2): inside a place the path's parts (Files, the place, each folder) are targets the bar's height;
# a tap on one opens it, a tap on the folder shown opens the Go to sheet, whose rows open the places and the
# storage's other top folders. The path bar is Files' alone: the Library's own folders are as they were.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Sorted
wait 300
tap library files.item.Sub
wait 300
tap library files.item.Deep
wait 300
expect files.place is documents/Sorted/Sub/Deep
expect files.crumbs is Files > Documents > Sorted > Sub > Deep
expect files.item.inside.pdf is inside.pdf
shot 01-path-bar
tap library files.crumb.3
wait 300
expect files.place is documents/Sorted
expect files.crumbs is Files > Documents > Sorted
tap library files.crumb.2
wait 300
expect files.place is documents
expect files.card is
tap library files.crumb.2
wait 300
expect files.card is goto
expect files.card.details is Go to
expect files.card.rows is Import|Documents|Downloads|Templates|Photos|Fonts|Kindle fonts|PDF copies|Trash|All storage|Notes|Cancel
shot 02-go-to
tap library files.goto.11
wait 300
expect files.card is
expect files.place is Notes
expect files.crumbs is Files > All storage > Notes
tap library files.crumb.3
wait 300
expect files.card is goto
tap library files.goto.4
wait 300
expect files.place is grimoire/templates
expect files.crumbs is Files > Templates
tap library files.crumb.1
wait 300
expect files.place is root
expect files.crumbs is
tap library files.back
wait 300
expect files.open is 0
tap library folder.School
wait 300
expect library.items is 1
expect files.open is 0
shot 03-library-folder
quit
