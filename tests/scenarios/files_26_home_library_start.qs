# root: files_25_home_library
# storage: 1
# The Library folder files_25_home_library made home, and renamed, after a restart (2.2, issue #160): Grimoire opens
# at Uni. Home is where the main menu opens, not where it comes back to: up to the top, a notebook opened and closed
# comes back to the top, where the user was; the home action (H) goes to Uni.
# rust-only
wait 300
expect settings.home is notebooks/Uni
expect library.folder is Uni
expect files.open is 0
expect toast is
shot 01-opens-at-home
tap library back
wait 300
expect library.folder is
tap library notebook.20261101-000000-0001
wait 600
expect screen is editor
pentap panel library
wait 600
expect screen is library
expect library.folder is
expect files.open is 0
key 35 0
wait 300
expect library.folder is Uni
quit
