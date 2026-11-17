# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# rust-only
# "Exit Grimoire?" over Files at home (2.2, issue #160) stays up through a turn of the screen and a sleep and wake,
# drawn whole with its answers the targets, and its No still answers it: Files at home as it was.
wait 300
expect files.at_home is 1
key 1 0
wait 300
expect library.sheet is exit
rotate 1
wait 500
expect library.sheet is exit
expect files.open is 1
expect library.ui_has.exit.no is 1
shot 01-turned
sleep
wait 500
wake
wait 500
expect library.sheet is exit
expect library.ui_has.exit.no is 1
shot 02-woke
tap library exit.no
wait 300
expect library.sheet is none
expect files.open is 1
expect files.place is documents/Sorted
rotate 0
wait 500
expect files.place is documents/Sorted
quit
