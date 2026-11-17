# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# The home folder renamed over USB while a file's card in it is open (2.2, issue #160): the card's Import finds the
# file changed ("File changed. Try again.") and Files reads the folder shown again; it is gone, so Files goes to the
# nearest folder above that opens (documents), not a stale screen. Nothing is imported.
# rust-only
wait 300
expect files.place is documents/Sorted
expect files.at_home is 1
storage move documents/Sorted documents/Was
tap library files.item.a.pdf
wait 300
expect files.card is pdf
tap library files.card.import
wait 500
expect toast is File changed. Try again.
expect files.place is documents
expect files.at_home is 0
expect library.ui_has.files.back is 1
quit
