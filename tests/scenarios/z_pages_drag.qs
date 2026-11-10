# rust-only
# generate: tests/drag_pages_fixture.py
# Drag to reorder (owner, 2026-11-02): a page held until its card opens and then moved without lifting is
# picked up (every hold here is 700 ms, the card's path; since 2026-11-04 a 300 ms hold and a move off the tile
# pick it up with no card, z_pages_drag_hold). Only the gap's black bar shows while it is dragged (the owner, 2026-11-03: "no need to have the
# page outline show when dragging, just show the black bar"; small fast refreshes of the bar alone), the
# lift drops it there through Move (one undo step) and draws the view once. A tile held and lifted still
# opens its card; a drop back in its own place, on the top bar or the bottom row, or a lost lift moves
# nothing and makes no undo step (in its own place the bar goes on one fast refresh, off the grid it is gone
# already, and a lost lift draws the view again). Over the bottom row (or the top bar) the screen turns. A ticked tile
# carries the ticked pages; a pen drags as a finger does.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
tap panel pages
waitbg
wait 200
expect editor.modal is pages
# Held and lifted without moving: the card, as before
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
expect editor.pages.drag is
tap panel pages.cancel
wait 100
expect editor.pages.context is 0
# Page 2 held, then dragged to the right half of page 6: after it
touch down 930 551
wait 700
expect editor.pages.context is 2
expect display.presents not -1
touch move 1000 640
expect editor.pages.context is 0
expect editor.pages.drag is 2 before 3
expect display.presents is ui 430 214 1000 1715
# The same gap (page 2's right half, then page 3's left half): the bar stays, nothing is refreshed
touch move 1040 700
expect display.presents is
touch move 1300 700
expect editor.pages.drag is 2 before 3
expect display.presents is
# Another gap: the bar's old place and its new one, each on its own fast refresh (nothing between them)
touch move 1600 1249
expect editor.pages.drag is 2 before 7
expect display.presents is fast 1230 214 12 674, fast 1842 912 12 674
shot 01-dragging
touch up 1600 1249
wait 200
expect editor.pages.drag is
expect editor.page_ids is 1,3,4,5,6,2,7,8,9,10,11,12
expect editor.pages.undo is 1
expect toast is
expect display.presents is full 0 0 1860 2480
shot 02-dropped
tap panel pages.undo-move
wait 200
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
expect editor.pages.undo is 0
# Page 5 to the left half of page 1: before it
touch down 930 1249
wait 700
touch move 900 1100
touch move 200 551
expect editor.pages.drag is 5 before 1
touch up 200 551
wait 200
expect editor.page_ids is 5,1,2,3,4,6,7,8,9,10,11,12
expect editor.page is 2
tap panel pages.undo-move
wait 200
# Back in its own place (page 3 dropped on the right half of page 2, before itself): nothing, no undo step, and
# only the bar taken off, on one fast refresh of its place (since 2026-11-04: the view was drawn whole again)
touch down 1542 551
wait 700
touch move 1400 600
touch move 1100 551
expect editor.pages.drag is 3 before 3
expect display.presents not -1
touch up 1100 551
wait 200
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
expect editor.pages.undo is 0
expect editor.pages.context is 0
expect display.presents is fast 1230 214 12 674
expect display.stale.1224.214.24.674 is 0
# Lifted over the top bar: nothing
touch down 318 551
wait 700
touch move 318 400
touch move 318 100
expect editor.pages.drag is 1 off
touch up 318 100
wait 200
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
expect editor.pages.undo is 0
# A lost lift (the next contact's down comes first): the drag is cancelled, nothing moves
touch down 318 1249
wait 700
touch move 400 1300
touch move 1542 1947
expect editor.pages.drag is 4 before 10
touch down 1542 1947
expect editor.pages.drag is
expect editor.page_ids is 1,2,3,4,5,6,7,8,9,10,11,12
touch up 1542 1947
wait 600
expect editor.pages.undo is 0
# Over the bottom row the screen turns; past the last tile of the second screen: the end
expect editor.pages.screen is 1
touch down 318 551
wait 700
touch move 318 700
touch move 900 2420
wait 900
expect editor.pages.screen is 2
touch move 930 1947
expect editor.pages.drag is 1 end
shot 03-turned
touch up 930 1947
wait 200
expect editor.page_ids is 2,3,4,5,6,7,8,9,10,11,12,1
expect editor.pages.screen is 2
expect editor.pages.undo is 1
# and back over the top bar to the first screen (page 12 now: the one moved) to before page 2
touch down 1542 551
wait 700
touch move 1542 700
touch move 900 100
wait 900
expect editor.pages.screen is 1
touch move 800 551
expect editor.pages.drag is 12 before 2
touch up 800 551
wait 200
expect editor.page_ids is 2,1,3,4,5,6,7,8,9,10,11,12
# Ticked pages 4 and 6: holding page 6 carries both, in their order
tap panel pages.multi
tap panel pages.page.4
tap panel pages.page.6
expect editor.ticked is 2
touch down 1542 1249
wait 700
touch move 1400 1100
touch move 200 551
expect editor.pages.drag is 4,6 before 1
touch up 200 551
wait 200
expect editor.page_ids is 4,6,2,1,3,5,7,8,9,10,11,12
expect editor.ticked is 0
tap panel pages.undo-move
wait 200
expect editor.page_ids is 2,1,3,4,5,6,7,8,9,10,11,12
# A pen drags too: page 9 to before page 8
pen down 1542 1947
wait 700
pen move 1400 1947
pen move 800 1947
expect editor.pages.drag is 9 before 8
pen up 800 1947
wait 200
expect editor.page_ids is 2,1,3,4,5,6,7,9,8,10,11,12
# The drop lands in the gap the bar shows: moves under 4 px are not drawn, so a lift 3 px past a tile's
# centre line (x 930) still drops before it, where the bar is (page 4 to before page 2)
touch down 318 1249
wait 700
touch move 600 900
touch move 928 551
expect editor.pages.drag is 4 before 2
touch up 931 551
wait 200
expect editor.page_ids is 2,4,1,3,5,6,7,9,8,10,11,12
tap panel pages.undo-move
wait 200
expect editor.page_ids is 2,1,3,4,5,6,7,9,8,10,11,12
# A drop whose move cannot be saved (the storage full): the view is drawn again (no gap bar
# left on the screen) and the ticks stay as they were (pages 4 and 6 ticked, page 2 dragged alone)
expect editor.pages.multi is 1
tap panel pages.page.4
tap panel pages.page.6
expect editor.ticked is 2
storage full
touch down 930 551
wait 700
touch move 1000 640
touch move 1600 1249
expect editor.pages.drag is 2 before 7
expect display.presents not -1
touch up 1600 1249
wait 200
expect toast has Couldn't move pages.
expect editor.pages.drag is
expect editor.page_ids is 2,1,3,4,5,6,7,9,8,10,11,12
expect editor.ticked is 2
expect editor.pages.undo is 0
expect display.presents has ui 0 0 1860 2480
storage ok
quit
