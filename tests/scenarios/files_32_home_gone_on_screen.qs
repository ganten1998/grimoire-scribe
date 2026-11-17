# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# The home folder renamed over USB while Files shows it (2.2, issue #160). A tap on one of its folder rows says
# "Couldn't open folder." and Files reads the folder shown again: it is gone, so Files goes to the nearest folder
# above that opens (documents, with its Back button), not a stale screen. The home action (H) then finds home gone:
# the Library's top, "Home folder not found.", and home is Notebooks again.
# rust-only
wait 300
expect files.place is documents/Sorted
expect files.at_home is 1
expect library.ui_has.files.back is 0
storage move documents/Sorted documents/Was
tap library files.item.Sub
wait 300
expect toast is Couldn't open folder.
expect files.place is documents
expect files.at_home is 0
expect library.ui_has.files.back is 1
expect files.order is Was
key 35 0
wait 300
expect files.open is 0
expect library.folder is
expect toast is Home folder not found.
expect settings.home is
quit
