# root: files_20_home_start
# storage: 1
# Home put back to Notebooks by files_20_home_start (2.2, issue #160), which checks that the Reset itself saved it:
# Grimoire opens at the Library's top again, as before a home was set, with nothing said.
# rust-only
wait 300
expect screen is library
expect settings.home is
expect files.open is 0
expect library.sheet is none
expect library.folder is
expect library.items is 4
expect toast is
quit
