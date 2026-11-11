# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# Resize a lasso selection (issue #37): a selection of strokes shows a handle at each corner of its box;
# dragging the bottom-right one scales the ink about the opposite corner, the box following it, one undo
# step at the lift, and undo puts the page back exactly; a tap on a handle that never moves changes nothing.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
# a ballpoint word and a neighbour
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.tool is ballpoint
pen curve 450 700 40 60 0.6 3
pen line 350 820 750 820 40 0.6
pen line 1100 700 1400 820 40 0.6
wait 100
expect editor.strokes is 3
# Select mode from the tool picker, a lasso around the word
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
pen down 280 580
pen move 500 570
pen move 820 580
pen move 830 750
pen move 820 900
pen move 500 910
pen move 280 900
pen move 270 750
pen up 275 600
wait 100
expect editor.select is selected
expect editor.selected is 2
expect editor.strokes is 3
expect editor.sel_box is 345 655 470 171
expect editor.page_sig_keep is kept
expect editor.view is 1.00 0 0
expect editor.scale is 1.00
expect editor.live_matches_render is 1
shot 01-selected-handles
# a tap on the bottom-right handle that never moves: no step, the box as it was
pen down 815 826
pen up 817 828
wait 100
expect editor.select is selected
expect editor.sel_box is 345 655 470 171
expect display.presents not -1
# dragged out by its width and height: the ink scaled about the top-left corner, one step at the lift
pen down 815 826
pen move 915 901
wait 150
expect display.presents is fast 10 640 1450 346
expect editor.sel_box is 345 655 583 212
pen move 1015 976
wait 150
expect display.presents is fast 330 640 728 284
expect editor.select is dragging
expect editor.sel_box is 345 655 698 254
expect editor.drag_inked is 1
pen up 1015 976
wait 100
expect editor.select is selected
expect editor.undo_kind is lasso
expect editor.sel_box is 345 655 698 254
expect editor.strokes is 3
expect editor.live_matches_render is 1
shot 02-resized
# undo puts the page back exactly, redo scales it again
tap panel select.done
wait 100
expect editor.select is off
pentap panel undo
wait 100
expect editor.page_sig_same is 1
pentap panel redo
wait 100
expect editor.page_sig_same is 0
expect editor.live_matches_render is 1
quit
