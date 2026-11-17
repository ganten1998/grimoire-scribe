# settings: status_mode=1
# settings: files_view=covers
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# A start at a home inside Files in the Covers view (2.2, issue #160): the first screen asks for no preview until
# the user first reaches the main menu (its tiles show the hourglass, and nothing is made), then they are asked for
# as when the folder is opened by hand: two PDFs, a picture and a font.
# rust-only
wait 300
expect files.open is 1
expect files.view is covers
expect files.place is documents/Sorted
expect library.home.pending is 1
wait 2000
expect files.asked is 0
expect files.tiles is illll
touch down 1000 2380
touch up 1000 2380
wait 300
expect library.home.pending is 0
waitbg
wait 2500
expect files.asked is 4
expect files.place is documents/Sorted
quit
