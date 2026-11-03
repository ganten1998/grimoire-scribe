# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# The lasso's live preview (docs/beta2/PIXEL-LASSO.md 2): while the lasso is drawn, the ink it would
# select if lifted now gets a halo, which follows the lasso as it goes round (none, one word, two, three);
# each pause folds the new points in once, with a few small fast refreshes (counted: the clock corner held
# still); the lift reuses the preview's answer and draws every place the preview refreshed again in grays.
# The page drawn live equals a fresh drawing throughout.
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
# three words in a row and one below
pen line 300 700 500 720 20 0.6
pen line 700 700 900 720 20 0.6
pen line 1100 700 1300 720 20 0.6
pen line 300 1200 600 1200 20 0.6
wait 100
expect editor.strokes is 4
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
# (Select mode's toast gone first: its redraw would land among the counted refreshes)
wait 2600
expect display.presents not clear
# the lasso, a pause after each stretch: a line selects nothing
pen down 250 600
pen move 600 600
pen move 1000 600
wait 200
expect editor.select is lasso
expect editor.halo is 0
expect editor.preview_batches is 1
expect editor.preview_refreshes is 0
# down the right side: the polygon closed back to the start takes the second word
pen move 1000 700
pen move 1000 800
wait 200
expect editor.halo is 1
expect editor.preview_batches is 2
expect editor.preview_refreshes is 1
# along the bottom: the first word too
pen move 600 800
pen move 250 800
wait 200
expect editor.halo is 2
expect editor.preview_batches is 3
expect editor.preview_refreshes is 2
# down and out under the last word: three
pen move 250 1050
pen move 250 1300
pen move 700 1300
wait 200
expect editor.halo is 3
expect editor.preview_batches is 4
expect editor.preview_refreshes is 3
shot 01-preview
expect editor.live_matches_render is 1
# the lasso's own stretches, and one small refresh per word that took its halo
expect display.presents is fast 243 593 766 15, fast 993 593 16 215, fast 690 690 219 41, fast 243 793 766 15, fast 290 690 219 41, fast 243 793 465 515, fast 290 1190 319 21
pen up 700 1300
wait 100
expect editor.select is selected
expect editor.selected is 3
expect editor.halo is 3
expect editor.preview_reused is 1
expect editor.lift_covers_preview is 1
shot 02-selected
expect editor.live_matches_render is 1
# one refresh in grays over the lasso, the halos and the bar
expect display.presents is ui 10 593 1450 1847
# a quick lasso with no pause: the lift answers alone, from the preview folded at the lift
tap panel select.done
wait 100
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 1050 600
pen move 1400 600
pen move 1400 800
pen move 1050 800
pen up 1050 650
wait 100
expect editor.select is selected
expect editor.selected is 1
expect editor.preview_reused is 1
expect editor.live_matches_render is 1
quit
