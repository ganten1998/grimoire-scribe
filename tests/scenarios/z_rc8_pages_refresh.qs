# rust-only
# generate: tests/page_management_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# RC8: the Pages view's previews come from the background job, and the view repaints only what changed.
# The job's previews arrive as one GL16 of the tile grid (never the whole screen, never twice for the
# same previews); a tap that selects a tile refreshes that tile (and the bottom row when its buttons
# change), and the second tap of a double tap still opens the page.
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
wait 3600
tap library folder.Void
tap library notebook.A
swipe 2 930 900 0 400
tap panel map.pages
expect editor.modal is pages
expect display.presents not -1
waitbg
wait 1200
expect editor.pages.cached is 9
expect display.presents is ui 24 214 1812 2070
wait 1200
expect display.presents is
shot 01-previews
# The first tick lights Copy page and Delete: the tile, then the bottom row, each on its own.
tap panel pages.page.2
expect editor.ticked is 1
expect display.presents is ui 636 214 588 674, ui 0 2310 1860 170
# Another tile: the two tiles alone.
tap panel pages.page.3
expect editor.ticked is 1
expect display.presents is ui 636 214 1200 674
shot 02-one-tile
tap panel pages.page.3
expect editor.modal is none
expect editor.page is 3
# Link to...'s notebook picker keeps the previews it decoded, and its job's previews repaint its tiles alone.
swipe 2 930 900 0 400
tap panel map.pages
touch down 318 550
wait 700
touch up 318 550
tap panel pages.link
pen line 1000 1100 1400 1300 20 0.6 0.6
pen leave
tap panel linkto.elsewhere
tap panel elsewhere.folder.Void/Nested
tap panel elsewhere.book.B
expect display.presents not -1
waitbg 5000
wait 1200
expect editor.destination.cached is 2
expect display.presents is ui 35 220 1333 325
shot 03-picker
tap panel elsewhere.page.2
expect editor.destination is 0
quit
