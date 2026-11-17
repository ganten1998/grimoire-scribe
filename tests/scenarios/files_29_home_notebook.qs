# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# Closing a notebook with a home inside Files (2.2, issue #160): Grimoire opens Files at documents/Sorted; out to the
# Library (Files' root, then its Back), a notebook opened and closed comes back to the Library as it was left, not to
# home; the home action (H) goes home.
# rust-only
wait 300
expect files.open is 1
expect files.place is documents/Sorted
expect files.at_home is 1
tap library files.crumb.1
wait 300
expect files.place is root
tap library files.back
wait 300
expect files.open is 0
expect library.folder is
tap library notebook.20261101-000000-0001
wait 600
expect screen is editor
pentap panel library
wait 600
expect screen is library
expect files.open is 0
expect library.folder is
key 35 0
wait 300
expect files.open is 1
expect files.place is documents/Sorted
expect files.at_home is 1
quit
