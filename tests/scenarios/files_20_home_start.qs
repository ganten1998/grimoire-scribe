# root: files_19_home_set
# storage: 1
# Home (2.2, issue #160) after a restart: files_19_home_set made documents/Sorted the home, so Grimoire opens Files
# there over the Library, with no Back button and nothing said. Reset home to Notebooks in its Go to sheet: Files
# stays where it is, its Back button is there again, the sheet offers Set as home, and settings.ini on disk has no
# home line at once (before quit saves the settings). files_21_home_reset starts here.
# rust-only
wait 300
expect screen is library
expect settings.home is files/documents/Sorted
expect settings.saved.home is files/documents/Sorted
expect files.open is 1
expect files.mode is place
expect files.place is documents/Sorted
expect files.crumbs is Files > Documents > Sorted
expect files.at_home is 1
expect library.ui_has.files.back is 0
expect library.folder is
expect toast is
shot 01-opens-at-home
tap library files.crumb.3
wait 300
expect files.card.rows is Home: Sorted|Import|Documents|Downloads|Templates|Photos|Fonts|Kindle fonts|PDF copies|Trash|All storage|Notes|Reset home to Notebooks|Cancel
tap library files.card.resethome
wait 300
expect toast is
expect files.card is
expect settings.home is
expect settings.saved.home is
expect files.home is
expect files.at_home is 0
expect files.place is documents/Sorted
expect library.ui_has.files.back is 1
shot 02-reset
tap library files.crumb.3
wait 300
expect files.card.rows is Home: Notebooks|Import|Documents|Downloads|Templates|Photos|Fonts|Kindle fonts|PDF copies|Trash|All storage|Notes|Set as home|Cancel
tap library files.card.cancel
wait 300
quit
