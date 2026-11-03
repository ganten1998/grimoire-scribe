# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# The lasso's other mode with a keyboard paired with the Kindle (docs/beta2/PIXEL-LASSO.md 1.10): Shift held
# as a lasso begins flips its mode. Shift down: this lasso in exact area cuts the line it crosses; Shift up:
# the next one takes it whole again, the mode itself never changed. Alt flips it too.
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
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
expect editor.sel_exact is 0
# Shift down (left Shift, 42, with Shift held), the lasso, Shift up
key 42 2
wait 50
pen down 700 600
pen move 1500 600
pen move 1500 800
pen move 700 800
pen up 700 650
wait 100
keyup 42 0
wait 50
expect editor.select is selected
expect editor.sel_exact is 1
expect editor.sel_cut is 1
expect editor.selected is 1
tap panel select.done
wait 100
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.sel_exact is 0
# Shift let go: whole strokes, the whole line
pen down 700 600
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
# Alt's own key events (left Alt, 56): exact area for this lasso too
key 56 4
wait 50
pen down 700 600
pen move 1500 600
pen move 1500 800
pen move 700 800
pen up 700 650
wait 100
keyup 56 0
wait 50
expect editor.sel_exact is 1
expect editor.sel_cut is 1
quit
