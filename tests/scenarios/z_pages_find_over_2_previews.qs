# rust-only
# generate: tests/drag_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_BG_DELAY_MS=1000
# Page previews that come while the Find sheet is open over the Pages view: they are
# not drawn under the sheet one by one, and as it closes the grid's tiles are refreshed with what it covered, one
# Ui, so the tiles above it show the new previews too (not only where the sheet was).
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
# (the clock corner held still: its minute's refresh would land among the counted ones)
# (each background job held back 1 s, GRIMOIRE_BG_DELAY_MS: the job's child writes the previews by the real clock
# while the app goes on with the round that opened the view, which draws the tiles just after starting the job. A
# preview written before its tile was drawn showed at once, so Cancel's Ui left that tile out, issue #184. Held
# back, every preview is written while the sheet is up.)
clock 1700000000
wait 3600
tap library folder.Void
tap library notebook.A
tap panel pages
expect editor.modal is pages
# Go to page... at once, while the previews are still being made
tap panel pages.goto
wait 100
expect editor.find is go
expect editor.modal is find
expect display.presents not -1
waitbg
wait 1200
# nothing drawn under the sheet as they came
expect display.presents is
# Cancel: the sheet's area and the grid's tiles, one Ui
tap 217 2365
wait 300
expect editor.find is 0
expect editor.modal is pages
expect editor.pages.cached is 9
expect display.presents is ui 20 214 1820 2246
shot 01-previews-after-find
# opened and closed again with no new previews: only what the sheet covered (its card with no rows yet)
tap panel pages.goto
wait 300
expect display.presents not -1
tap 217 2365
wait 300
expect display.presents is ui 20 1480 1820 980
quit
