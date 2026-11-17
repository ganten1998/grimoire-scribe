# root: files_27_home_rename
# storage: 1
# The folder files_27_home_rename made home and renamed, after a restart (2.2, issue #160): Grimoire opens Files at
# grimoire/photos/Trip. Deleted from its card in Photos (into the trash), it is gone: the home action (H) shows the
# Library's top with the one toast, "Home folder not found.", and home is Notebooks again.
# rust-only
wait 300
expect settings.home is files/grimoire/photos/Trip
expect files.open is 1
expect files.place is grimoire/photos/Trip
expect files.at_home is 1
expect toast is
tap library files.crumb.2
wait 300
expect files.place is grimoire/photos
tap library files.more.1
wait 300
expect files.card.rows is Open|Import 1 picture as notebook|Reset home to Notebooks|Rename|Delete|Cancel
tap library files.card.trash
wait 500
expect files.card is delete
tap library files.card.delete
wait 300
expect storage.exists.grimoire/photos/Trip is 0
expect settings.home is files/grimoire/photos/Trip
key 35 0
wait 300
expect files.open is 0
expect library.folder is
expect toast is Home folder not found.
expect settings.home is
shot 01-gone
quit
