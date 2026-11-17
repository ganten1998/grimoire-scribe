# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# A folder of Files as home from its card (2.2, issue #160): grimoire/photos/Scans' card has Set as home, and once it
# is home, Reset home to Notebooks in its place. Renamed with the card's Rename, it takes home with it: settings.ini
# names grimoire/photos/Trip, and the home action (H) goes there. files_28_home_rename_start starts from here.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.Photos
wait 300
expect files.place is grimoire/photos
expect files.item.Scans is Scans
tap library files.more.1
wait 300
expect files.card is folder
expect files.card.rows is Open|Import 1 picture as notebook|Set as home|Rename|Delete|Cancel
shot 01-folder-card
tap library files.card.sethome
wait 300
expect files.card is
expect toast is
expect settings.home is files/grimoire/photos/Scans
tap library files.more.1
wait 300
expect files.card.rows is Open|Import 1 picture as notebook|Reset home to Notebooks|Rename|Delete|Cancel
shot 02-folder-card-home
tap library files.card.rename
wait 300
expect files.keyboard is 1
expect files.text is Scans
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
expect files.text is
tap 175 2205
tap 841 1885
tap 664 1885
tap 1372 1885
tap 1726 1885
expect files.text is Trip
tap 1685 2365
wait 300
expect files.keyboard is 0
expect storage.exists.grimoire/photos/Trip is 1
expect storage.exists.grimoire/photos/Scans is 0
expect settings.home is files/grimoire/photos/Trip
expect files.home is files/grimoire/photos/Trip
tap library files.back
wait 300
expect files.place is root
key 35 0
wait 300
expect files.place is grimoire/photos/Trip
expect files.at_home is 1
expect library.ui_has.files.back is 0
quit
