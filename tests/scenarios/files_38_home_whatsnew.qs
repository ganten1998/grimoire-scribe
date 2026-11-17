# settings: settings_version=3
# settings: whatsnew_seen=1.2.0
# settings: status_mode=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# rust-only
# The first start after an update with a home inside Files (2.2, issue #160): What's new shows over the Library;
# closed, Files opens at home.
wait 300
expect library.sheet is about
expect files.open is 0
tap library about.close
wait 300
expect library.sheet is none
expect files.open is 1
expect files.place is documents/Sorted
expect files.at_home is 1
quit
