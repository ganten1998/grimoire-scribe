# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# A folder's card checks it is still the folder it was opened for, by its identity, not just a folder at its path
# (2.2, issue #144): with Trip's card open in grimoire/photos, a writer over USB (the runner's storage move) renames
# Trip away and Other to Trip. Rename, Delete (before its question, and at the question's own Delete) and Import 1
# picture as notebook each say "File changed. Try again.", act on nothing, and the folder is read again.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.Photos
wait 300
expect files.order is Other|Trip|beach.jpg
tap library files.more.2
wait 300
expect files.card is folder
expect files.card.rows is Open|Import 1 picture as notebook|Rename|Delete|Cancel
storage move grimoire/photos/Trip grimoire/photos/Was
storage move grimoire/photos/Other grimoire/photos/Trip
tap library files.card.rename
wait 300
expect toast is File changed. Try again.
expect files.keyboard is 0
expect files.card is
expect files.order is Trip|Was|beach.jpg
shot 01-refused
# the same for Delete, on the card of the Trip there now: refused before its question is asked
tap library files.more.1
wait 300
storage move grimoire/photos/Trip grimoire/photos/Other
storage move grimoire/photos/Was grimoire/photos/Trip
tap library files.card.trash
wait 300
expect toast is File changed. Try again.
expect files.card is
expect files.order is Other|Trip|beach.jpg
expect storage.names.grimoire/trash is 20261101-000000-0001-p3-1789995000|20261101-000000-0004-1789996000
# and when the folder is swapped while the question is up: its Delete moves nothing
tap library files.more.2
wait 300
tap library files.card.trash
wait 500
expect files.card is delete
expect files.card.details is Delete "Trip"?
storage move grimoire/photos/Trip grimoire/photos/Was
storage move grimoire/photos/Other grimoire/photos/Trip
tap library files.card.delete
wait 300
expect toast is File changed. Try again.
expect files.order is Trip|Was|beach.jpg
expect storage.names.grimoire/trash is 20261101-000000-0001-p3-1789995000|20261101-000000-0004-1789996000
storage move grimoire/photos/Trip grimoire/photos/Other
storage move grimoire/photos/Was grimoire/photos/Trip
tap library files.back
wait 300
tap library files.item.Photos
wait 300
expect files.order is Other|Trip|beach.jpg
# and for the import of its pictures: nothing is imported
tap library files.more.2
wait 300
storage move grimoire/photos/Trip grimoire/photos/Was
storage move grimoire/photos/Other grimoire/photos/Trip
tap library files.card.pictures
wait 300
expect toast is File changed. Try again.
expect files.open is 1
expect bg.busy is 0
expect storage.names.grimoire/photos is README.txt|Trip|Was|beach.jpg
quit
