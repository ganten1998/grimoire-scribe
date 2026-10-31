# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/link-keys .
# A key on the Pages view's keyboard costs what a key on the Library's does: the text line, then the
# key, each its own fast refresh, the keyboard drawn once; nothing for the pen's samples while it
# rests on the key (one a digitizer frame on the Kindle); the key alone at its release; one GL16 of
# the keyboard once typing pauses, and nothing of the view under it. It used to send the text line
# and the key as one fast refresh from the text line down to the key, which on the Kindle waits in
# the driver's queue behind the key before it, and to draw the whole keyboard again at every sample.
# A settle due on the page when the Pages view opened (Name the page, N, right after a stroke) no
# longer draws the view again under the keyboard and the panel over both. Name the page still does what it
# did. Go to page... is the Find sheet now: a key there costs the same, with the card of rows it offers
# refreshed between the text line and the key (the split keyboard host). (display.presents and keyboard.draws
# count from their last check: a "not -1" check only starts the count.)
# (past the two repaints just after the start, main.c, so the checks see only what the keys send)
# rust-only
wait 3600
# The Library's keyboard (New folder): a with the pen, resting on it for eight samples
tap 1155 95
wait 100
expect library.sheet is keyboard
expect display.presents not -1
expect keyboard.draws not -1
pen down 221 2045 0.3
expect display.presents is fast 50 1510 1760 120, fast 138 1970 167 150
expect keyboard.draws is 1
pen move 221 2046 0.5
pen move 222 2046 0.6
pen move 222 2047 0.6
pen move 222 2047 0.5
pen move 223 2047 0.4
pen move 223 2048 0.3
pen move 223 2048 0.2
pen move 223 2048 0.1
expect display.presents is
expect keyboard.draws is 0
pen up 223 2048
expect display.presents is fast 138 1970 167 150
expect keyboard.draws is 1
wait 1000
expect display.presents is ui 20 1480 1820 980
expect keyboard.draws is 0
expect library.text is a
tap 217 2365
wait 300
expect library.sheet is none
# A notebook, its first stroke, and Name the page at once: the settle the stroke left is not drawn
# under the Pages view (no GL16 of the stroke and the panel, no keyboard drawn again)
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen line 300 600 900 620 30 0.5 0.9
key 49 0
wait 60
expect editor.modal is pages
expect editor.keyboard is Page name
expect display.presents not -1
expect keyboard.draws not -1
wait 2000
expect display.presents is
expect keyboard.draws is 0
shot 01-name-keyboard-after-a-stroke
# The same a on the Pages view's keyboard, the same refreshes
pen down 221 2045 0.3
expect display.presents is fast 50 1510 1760 120, fast 138 1970 167 150
expect keyboard.draws is 1
pen move 221 2046 0.5
pen move 222 2046 0.6
pen move 222 2047 0.6
pen move 222 2047 0.5
pen move 223 2047 0.4
pen move 223 2048 0.3
pen move 223 2048 0.2
pen move 223 2048 0.1
expect display.presents is
expect keyboard.draws is 0
pen up 223 2048
expect display.presents is fast 138 1970 167 150
expect keyboard.draws is 1
wait 1000
expect display.presents is ui 20 1480 1820 980
expect keyboard.draws is 0
# The space bar with a finger: the key far below the text line is still its own refresh
touch down 1060 2365
expect display.presents is fast 50 1510 1760 120, fast 841 2290 355 150
expect keyboard.draws is 1
touch up 1060 2365
expect display.presents is fast 841 2290 355 150
expect keyboard.draws is 1
tap 1679 2205
tap 398 2045
expect editor.text is as
shot 02-name-typed
# Done saves the name and goes back to the page, as before
tap 1643 2365
expect editor.modal is none
expect editor.keyboard is
expect editor.page_name is as
expect editor.index_rows is 1
# Go to page... is the Find sheet now (a page number or a name): the view closes and Find opens over the page; a
# key costs what it does on every keyboard (the text line, then the key, each its own fast refresh) and, when the
# rows it offers change, the card of rows between the two; a number the notebook does not have is a line that is
# not a button, and Done leaves the sheet as it is
pentap panel pages
wait 400
expect editor.modal is pages
tap panel pages.goto
wait 300
expect editor.modal is find
expect editor.keyboard is Page number or name
expect display.presents not -1
expect keyboard.draws not -1
touch down 841 1725
expect display.presents is fast 50 1510 1760 120, fast 20 1312 1820 156, fast 758 1650 167 150
expect keyboard.draws is 1
touch up 841 1725
expect display.presents is fast 758 1650 167 150
expect keyboard.draws is 1
expect editor.find.1 is There is no page 5 (1 page).
tap 1643 2365
expect editor.keyboard is Page number or name
expect editor.text is 5
expect display.presents not -1
touch down 1679 2205
expect display.presents is fast 50 1510 1760 120, fast 20 1312 1820 156, fast 1549 2130 261 150
touch up 1679 2205
expect editor.text is
expect editor.find.rows is 0
tap 133 1725
expect editor.text is 1
expect editor.find.1 is Go to page 1
tap 1643 2365
wait 300
expect editor.modal is none
expect editor.page is 1
expect editor.keyboard is
