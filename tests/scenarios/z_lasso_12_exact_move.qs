# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# The exact-area lasso (docs/beta2/PIXEL-LASSO.md 1): with the bar's mode on Exact area, a lasso through
# the middle of a line of writing selects only the ink inside it: while it is drawn the halo covers the
# inside run alone; lifted, the line is cut there (the plan: nothing on the page changes yet). A drag moves
# the right part; the left part stays where it was, drawn exactly as before. One undo step puts the line
# back whole, redo cuts it again. The page drawn live equals a fresh drawing after every step.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.tool is ballpoint
pen line 300 700 1300 720 40 0.6
pen line 300 1000 600 1000 20 0.6
pen leave
wait 100
expect editor.strokes is 2
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
expect editor.sel_exact is 0
tap panel select.mode
wait 100
expect editor.select is armed
# a lasso round the right half of the line: the halo covers the inside run as it is drawn
pen down 800 600
pen move 1500 600
pen move 1500 800
wait 200
expect editor.sel_exact is 1
expect editor.halo is 1
pen move 800 800
pen up 800 650
wait 100
expect editor.select is selected
expect editor.selected is 1
expect editor.sel_cut is 1
expect editor.strokes is 2
expect editor.live_matches_render is 1
shot 01-cut
# drag the right part down
pen down 1100 710
pen move 1100 900
pen move 1100 1300
pen up 1100 1400
wait 100
expect editor.select is selected
expect editor.undo_kind is lasso
expect editor.strokes is 3
expect editor.sel_cut is 0
expect editor.live_matches_render is 1
shot 02-moved
tap panel select.done
wait 100
pentap panel undo
wait 100
expect editor.strokes is 2
expect editor.live_matches_render is 1
pentap panel redo
wait 100
expect editor.strokes is 3
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 2
expect editor.live_matches_render is 1
# exact area again, Delete: the left part stays, one step
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.sel_exact is 1
pen down 800 600
pen move 1500 600
pen move 1500 800
pen move 800 800
pen up 800 650
wait 100
expect editor.selected is 1
tap panel select.delete
wait 100
expect editor.strokes is 2
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 2
expect editor.live_matches_render is 1
quit
