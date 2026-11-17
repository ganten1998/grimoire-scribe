# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Nowhere
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# A home that is gone at the start (2.2, issue #160): settings.ini names documents/Nowhere, which is not there.
# Grimoire opens at the Library's top with one toast, "Home folder not found.", makes nothing at the path, and home
# is Notebooks again: the home action (H) then goes to the Library's top with nothing said.
# rust-only
wait 300
expect toast is Home folder not found.
expect settings.home is
expect files.open is 0
expect library.sheet is none
expect library.folder is
expect storage.exists.documents/Nowhere is 0
shot 01-not-found
wait 4300
expect toast is
tap library folder.School
wait 300
expect library.folder is School
key 35 0
wait 300
expect library.folder is
expect files.open is 0
expect toast is
quit
