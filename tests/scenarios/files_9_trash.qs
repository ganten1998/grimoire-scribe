# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# Delete a photo, then Restore it from the Trash place (2.2, issue #144). The root's Trash row shows the trash's
# size once its walk is through (7 KB: a notebook of 5 KB and a page of 2 KB), and 10 KB once the photo (3 KB) is
# in it. The photo's card moves it to the trash ("Moved to grimoire/trash"); the Trash place lists its entries
# newest first, each with its size and date (the photo by its own name, the notebook by its name, the page by its
# notebook's); the page has no card (pages are listed, never restored); the photo's card offers Restore alone, which
# puts it back where it was ("Restored to grimoire/photos.").
# rust-only
wait 300
tap library files-card
wait 300
expect files.order is Import|Templates|Photos|Fonts|Trash|All storage
waitfor files.sub.Trash is grimoire/trash, 7 KB
tap library files.item.Photos
wait 300
expect files.order is Other|Trip|beach.jpg
tap library files.item.beach.jpg
wait 300
expect files.card is picture
expect files.card.rows is Rename|Delete (moves it to grimoire/trash)|Cancel
tap library files.card.trash
wait 300
expect toast is Moved to grimoire/trash
expect files.card is
expect files.order is Other|Trip
expect storage.exists.grimoire/photos/beach.jpg is 0
shot 01-deleted
tap library files.crumb.1
wait 300
waitfor files.sub.Trash is grimoire/trash, 10 KB
shot 02-root
tap library files.item.Trash
wait 300
expect files.place is grimoire/trash
expect files.rows is 3
waitfor files.row.1 is beach.jpg
waitfor files.sub.1 has 3 KB,
waitfor files.row.2 is Beta
waitfor files.sub.2 has 5 KB,
waitfor files.row.3 is Alpha
waitfor files.sub.3 has 2 KB,
shot 03-trash
tap library files.row.3
wait 200
expect files.card is
tap library files.row.1
wait 300
expect files.card is trashed
expect files.card.details has beach.jpg, 3 KB,
expect files.card.rows is Restore|Cancel
shot 04-card
tap library files.card.restore
wait 300
expect toast is Restored to grimoire/photos.
expect files.rows is 2
expect files.row.1 is Beta
expect storage.size.grimoire/photos/beach.jpg is 3072
shot 05-restored
tap library files.crumb.1
wait 300
waitfor files.sub.Trash is grimoire/trash, 7 KB
tap library files.item.Photos
wait 300
expect files.order is Other|Trip|beach.jpg
quit
