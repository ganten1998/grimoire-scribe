# settings: status_mode=1
# settings: mirror_pdf=0
# settings: gesture_new_page=0
# rust-only
# The lasso's live preview (docs/beta2/PIXEL-LASSO.md 2) as ink comes in and goes out: a word the lasso
# takes in gets its halo, and loses it when the lasso goes on round a pocket that leaves it out; lifted
# then, nothing is selected (the preview said so). A finger's lasso (armed by its hold) shows the same
# halo as it goes, and selects what it showed. The page drawn live equals a fresh drawing throughout.
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
pen line 300 700 500 720 20 0.6
pen line 1100 700 1300 720 20 0.6
pen leave
wait 100
expect editor.strokes is 2
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
# a box around the first word: its halo comes
pen down 250 600
pen move 550 600
pen move 550 800
pen move 250 800
wait 200
expect editor.halo is 1
expect editor.preview_refreshes is 1
# on round a pocket that opens to the left, past the lasso's start: the word is outside again
pen move 250 780
pen move 520 780
pen move 520 640
pen move 260 640
wait 200
expect editor.halo is 0
expect editor.preview_refreshes is 2
expect editor.live_matches_render is 1
pen up 260 640
pen leave
wait 100
expect editor.select is armed
expect editor.selected is 0
expect editor.preview_reused is 1
expect editor.lift_covers_preview is 1
expect toast is Nothing to select.
tap panel select.done
wait 100
expect editor.select is off
# past the launch's whole-screen repaints and the toast
wait 3600
# a finger held still arms a lasso; drawn round the second word, its halo shows before the lift
finger 1 down 1050 600
frame 60
wait 800
expect editor.select is lasso
finger 1 move 1350 600
frame 20
finger 1 move 1350 800
frame 20
finger 1 move 1050 800
frame 20
wait 200
expect editor.halo is 1
finger 1 move 1050 650
frame 20
finger 1 up 1050 650
frame 20
wait 100
expect editor.select is selected
expect editor.selected is 1
expect editor.preview_reused is 1
expect editor.lift_covers_preview is 1
expect editor.live_matches_render is 1
quit
