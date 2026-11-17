# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# rust-only
# A home inside Files while the Files card is hidden (2.2, issue #160; files_card not set): Files opens at home. Out
# to the Library through the path bar and the root's Back: no Files card there. The home action (H) goes home again.
wait 300
expect files.open is 1
expect files.place is documents/Sorted
expect files.at_home is 1
expect library.ui_has.files.back is 0
tap library files.crumb.1
wait 300
expect files.place is root
tap library files.back
wait 300
expect files.open is 0
expect library.items is 3
key 35 0
wait 300
expect files.open is 1
expect files.place is documents/Sorted
quit
