# rust-only
# generate: tests/drag_pages_fixture.py
# The drag's own hold (2.0.0-beta.2, the owner's Scribe feedback of 2026-11-04: "Make the delay for reordering
# smaller, so that it can safely be activated before the long-pressed management dialong. They are too
# disruptive as a pair."): a tile held 300 ms (DRAG_HOLD_MS) and then moved past the slop is picked up with no
# card ever drawn, the gap's bar alone on one fast refresh, and the card's 600 ms passing under the drag draws
# nothing. A move before 300 ms stays what it was, and never turns into a drag later in the same contact: quick,
# it turns the screen; slow, it does nothing. Held still to 600 ms the card opens, as before (z_pages_drag holds
# through the card and drags from it). A ticked tile carries the ticked pages and a pen drags as a finger does,
# on the shorter hold too.
# (the clock corner held still: its minute's refresh would land among the counted ones; and the launch repaints
# the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 3600
clock 1700000000
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
tap panel pages
waitbg
wait 200
expect editor.modal is pages
expect editor.pages.screen is 1
# Page 2 held 350 ms: nothing drawn yet; then moved: dragged, no card, the bar's one fast refresh
expect display.presents not -1
touch down 930 551
wait 350
expect editor.pages.context is 0
expect editor.pages.drag is
expect display.presents is
touch move 1000 640
expect editor.pages.context is 0
expect editor.pages.drag is 2 before 3
expect display.presents is fast 1230 214 12 674
# Still dragging past the card's 600 ms: no card, nothing drawn
wait 400
expect editor.pages.context is 0
expect editor.pages.drag is 2 before 3
expect display.presents is
touch move 1600 1249
expect editor.pages.drag is 2 before 7
expect editor.pages.context is 0
shot 01-dragging
expect display.presents not -1
touch up 1600 1249
wait 200
expect editor.pages.drag is
expect editor.pages.context is 0
expect editor.page_ids is 1,3,4,5,6,2,7,8,9,10,11,12
expect editor.pages.undo is 1
expect display.presents is full 0 0 1860 2480
tap panel pages.undo-move
wait 200
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
# A swipe from a tile, moving at once, turns the screen (and back): no drag, no card
fpath 1400 1300 600 1320 200
wait 200
expect editor.pages.screen is 2
expect editor.pages.drag is
expect editor.pages.context is 0
fpath 400 1300 1200 1280 200
wait 200
expect editor.pages.screen is 1
# Moved past the slop at 100 ms, then on past 300 ms and lifted quickly: still a flick (the next screen), never a drag
touch down 930 551
wait 100
touch move 1000 551
expect editor.pages.drag is
wait 300
touch move 400 560
expect editor.pages.drag is
touch up 400 560
wait 200
expect editor.pages.drag is
expect editor.pages.context is 0
expect editor.pages.screen is 2
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
fpath 400 1300 1200 1280 200
wait 200
expect editor.pages.screen is 1
# The same slowly (lifted at 1 s, on page 6's tile): nothing at all, no drag, no card, no tick
touch down 930 551
wait 100
touch move 1000 551
wait 400
touch move 1300 900
expect editor.pages.drag is
wait 500
expect editor.pages.context is 0
touch move 1542 1249
expect editor.pages.drag is
touch up 1542 1249
wait 200
expect editor.pages.drag is
expect editor.pages.context is 0
expect editor.pages.screen is 1
expect editor.ticked is 0
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
# Held still to 600 ms: the card, as before, and no drag at its lift
touch down 930 551
wait 700
expect editor.pages.context is 2
touch up 930 551
wait 100
expect editor.pages.context is 2
expect editor.pages.drag is
tap panel pages.cancel
wait 100
expect editor.pages.context is 0
# Ticked pages 4 and 6: page 6 held 350 ms carries both, in their order, with no card
tap panel pages.multi
tap panel pages.page.4
tap panel pages.page.6
expect editor.ticked is 2
touch down 1542 1249
wait 350
touch move 1400 1100
expect editor.pages.context is 0
touch move 200 551
expect editor.pages.drag is 4,6 before 1
touch up 200 551
wait 200
expect editor.page_ids is 4,6,1,2,3,5,7,8,9,10,11,12
tap panel pages.undo-move
wait 200
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
# A pen held 350 ms drags too: page 9 to before page 8, no card
tap panel pages.multi
expect editor.ticked is 0
pen down 1542 1947
wait 350
pen move 1400 1947
expect editor.pages.context is 0
pen move 800 1947
expect editor.pages.drag is 9 before 8
pen up 800 1947
wait 200
expect editor.pages.context is 0
expect editor.page_ids is 1,2,3,4,5,6,7,9,8,10,11,12
quit
