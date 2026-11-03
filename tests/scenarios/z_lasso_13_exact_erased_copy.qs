# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# The exact-area lasso with erasing and the clipboard (docs/beta2/PIXEL-LASSO.md 1.3, 1.4): a white-ink
# gap in the part of a line the lasso cuts off travels with it when it moves, and the part that stays keeps
# its own gap (the white stroke stays where it was). Copy changes nothing on the page (its strokes, byte for
# byte, and its canvas); Paste puts down the cut part alone with its gap. The page drawn live equals a
# fresh drawing after every step.
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
pen line 300 700 1300 700 40 0.6
pen leave
wait 100
# white ink across the line twice: a gap in each half
tap panel tool-picker
tap panel picker.ink.1
wait 100
tap 100 2400
wait 100
expect settings.white is 1
pen line 500 650 520 760 10 0.9
pen line 1100 650 1120 760 10 0.9
pen leave
wait 100
tap panel tool-picker
tap panel picker.ink.0
wait 100
tap 100 2400
wait 100
expect settings.white is 0
expect editor.strokes is 3
# Select, exact area, a lasso round the right half (the second gap inside it)
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
tap panel select.mode
wait 100
pen down 800 600
pen move 1500 600
pen move 1500 800
pen move 800 800
pen up 800 650
wait 100
expect editor.select is selected
expect editor.selected is 1
expect editor.sel_cut is 1
# Copy: nothing on the page changes
expect editor.page_sig_keep is kept
tap panel select.copy
wait 100
expect editor.page_sig_same is 1
expect editor.strokes is 3
expect editor.sel_cut is 1
expect editor.live_matches_render is 1
# the move takes the second gap's white ink along (a copy of it); the left part keeps its gap
pen down 1000 700
pen move 1000 1000
pen up 1000 1300
wait 100
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
shot 01-moved
pentap panel undo
wait 100
expect editor.strokes is 3
expect editor.live_matches_render is 1
# paste the copy: the cut part alone with its gap (a copy of its white ink, scoped by a floor marker so it
# never lightens the line under it), 3 mm off while the line is whole
tap panel select.paste
wait 100
expect editor.select is selected
expect editor.undo_kind is lasso
expect editor.strokes is 6
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 3
expect editor.page_sig_same is 1
expect editor.live_matches_render is 1
quit
