# settings: status_mode=1
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/lasso-exact-keys .
# rust-only
# Choosing whole strokes or exact area with a paired keyboard as well as the pen (docs/beta2/PIXEL-LASSO.md
# 1.10): the `select-exact` action on a key (E here) starts Select mode in exact area; the live preview
# shows the inside run of the line the lasso crosses, with a tick where it will be cut; lifted, the line is
# cut. The bar's mode button judges the same lasso again at once (whole strokes: the whole line; exact area
# again: the cut), nothing on the page changing; the key toggles it too. Alt held as a lasso begins flips
# the mode for that lasso alone. Copy then Paste with a cut selection up selects the pasted part alone.
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
expect editor.strokes is 1
key 18 0
wait 100
expect editor.select is armed
expect editor.sel_exact is 1
pen down 700 600
pen move 1500 600
pen move 1500 800
pen move 700 800
wait 200
expect editor.halo is 1
shot 01-preview-tick
pen up 700 650
wait 100
expect editor.select is selected
expect editor.sel_cut is 1
expect editor.selected is 1
expect editor.page_sig_keep is kept
# the mode button: the same lasso in whole strokes takes the whole line (more than half of it inside)
tap panel select.mode
wait 100
expect editor.select is selected
expect editor.sel_exact is 0
expect editor.sel_cut is 0
expect editor.selected is 1
expect editor.page_sig_same is 1
# the key toggles too: exact area again
key 18 0
wait 100
expect editor.sel_exact is 1
expect editor.sel_cut is 1
expect editor.page_sig_same is 1
expect editor.live_matches_render is 1
tap panel select.done
wait 100
expect editor.select is off
# Alt held as the lasso begins: whole strokes for this lasso, the mode stays exact area
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.sel_exact is 1
mods 4
pen down 700 600
mods 0
pen move 1500 600
pen move 1500 800
pen move 700 800
pen up 700 650
wait 100
expect editor.select is selected
expect editor.sel_exact is 0
expect editor.sel_cut is 0
expect editor.selected is 1
tap panel select.done
wait 100
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.sel_exact is 1
# Copy then Paste with the cut selection up: the pasted part is selected, whole, with no plan left over
pen down 700 600
pen move 1500 600
pen move 1500 800
pen move 700 800
pen up 700 650
wait 100
expect editor.sel_cut is 1
tap panel select.copy
wait 100
tap panel select.paste
wait 100
expect editor.select is selected
expect editor.sel_cut is 0
expect editor.selected is 1
expect editor.strokes is 2
expect editor.live_matches_render is 1
quit
