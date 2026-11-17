# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# Home (2.2, issue #160), set from Files' Go to sheet: its first row names home (Notebooks until one is set), and
# Set as home, after the folders, makes the folder shown the home. Nothing is said: the folder that is home has no
# Back button, the sheet's first row names it, and Reset home to Notebooks stands where Set as home did. Esc there
# asks "Exit Grimoire?", as at the Library's top; the path bar still leads up. files_20_home_start starts from here.
# rust-only
wait 300
expect settings.home is
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Sorted
wait 300
expect files.place is documents/Sorted
expect files.at_home is 0
expect library.ui_has.files.back is 1
tap library files.crumb.3
wait 300
expect files.card is goto
expect files.card.rows is Home: Notebooks|Import|Documents|Downloads|Templates|Photos|Fonts|Kindle fonts|PDF copies|Trash|All storage|Notes|Set as home|Cancel
shot 01-go-to
tap library files.card.sethome
wait 300
expect toast is
expect files.card is
expect settings.home is files/documents/Sorted
expect files.home is files/documents/Sorted
expect files.at_home is 1
expect files.place is documents/Sorted
expect library.ui_has.files.back is 0
shot 02-home
tap library files.crumb.3
wait 300
expect files.card.rows is Home: Sorted|Import|Documents|Downloads|Templates|Photos|Fonts|Kindle fonts|PDF copies|Trash|All storage|Notes|Reset home to Notebooks|Cancel
shot 03-go-to-at-home
tap library files.card.cancel
wait 300
# Esc at home: the question, and No stays
key 1 0
wait 300
expect library.sheet is exit
expect files.open is 1
shot 04-exit
tap library exit.no
wait 300
expect library.sheet is none
expect files.open is 1
expect files.place is documents/Sorted
# a folder inside home: Back is there, and leads back to home, no further
tap library files.item.Sub
wait 300
expect files.at_home is 0
expect library.ui_has.files.back is 1
tap library files.back
wait 300
expect files.place is documents/Sorted
expect library.ui_has.files.back is 0
key 1 0
wait 300
expect library.sheet is exit
key 1 0
wait 300
expect library.sheet is none
# the path bar leads up, and from there the Go to sheet's first row leads home
tap library files.crumb.2
wait 300
expect files.place is documents
expect library.ui_has.files.back is 1
tap library files.crumb.2
wait 300
expect files.card is goto
expect files.card.rows has Home: Sorted|
expect files.card.rows has |Set as home|Reset home to Notebooks|Cancel
tap library files.goto.home
wait 300
expect files.place is documents/Sorted
expect files.at_home is 1
quit
