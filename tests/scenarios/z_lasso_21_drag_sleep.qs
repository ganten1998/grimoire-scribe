# settings: mirror_pdf=0
# rust-only
# generate: tests/lasso_drag_fixture.py
# The drag's two edges (2.1.0, the lasso drag's review): a tap inside the selection that wobbles a pixel is a tap
# (the pen stays within the drag's 1 mm slop: no show, the bar left alone, nothing moved), and a sleep in the
# middle of a drag ends it as a cancel, so the Kindle wakes to the box at its place with its bar under it and
# nothing moved. (Before, the drag outlived the sleep with its lift lost: the box at the moved place, no bar
# drawn, and the bar's buttons still answering at their place from before the drag, where a tap on nothing
# deleted the selection.) A drag after the wake moves it as any other.
clock 1700000000
wait 300
tap 326 608
wait 2000
expect screen is editor
expect editor.strokes is 12
tap panel tool-picker
tap panel picker.select
wait 100
pen down 440 440
pen move 580 430
pen move 720 440
pen move 730 570
pen move 720 700
pen move 580 710
pen move 440 700
pen move 430 570
pen up 440 450
wait 200
expect editor.select is selected
expect editor.selected is 12
expect editor.sel_box is 482 482 193 186
expect editor.page_sig_keep is kept
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 4000
expect display.presents not -1
# a tap that wobbles a pixel: nothing shown, nothing moved
pen down 600 600 0.5
pen move 601 600 0.5
pen move 601 601 0.5
wait 200
expect editor.select is dragging
expect editor.drag_samples is 0
expect editor.drag_shows is 0
expect display.presents is
pen up 601 601
wait 300
expect editor.select is selected
expect editor.sel_box is 482 482 193 186
expect editor.page_sig_same is 1
# a drag, and the Kindle sleeps in the middle of it
pen down 600 600 0.5
pen move 600 700 0.5
pen move 600 800 0.5
pen move 600 900 0.5
wait 200
expect editor.select is dragging
expect editor.drag_shows is 1
expect editor.drag_shown is 0 300
sleep
wait 200
expect asleep is 1
wake
wait 400
expect asleep is 0
expect editor.select is selected
expect editor.sel_box is 482 482 193 186
expect editor.drag_copy is 0
expect editor.page_sig_same is 1
expect editor.layers is ok
shot 01-woken
# a drag after the wake: as any other, one undo step
pen down 600 600 0.5
pen move 600 700 0.5
pen move 600 800 0.5
pen move 600 900 0.5
wait 200
expect editor.select is dragging
expect editor.drag_shows is 1
pen up 600 900
wait 300
expect editor.select is selected
expect editor.sel_box is 482 782 193 186
expect editor.undo_kind is lasso
expect editor.page_sig_same is 0
expect editor.live_matches_render is 1
expect editor.layers is ok
shot 02-moved
quit
