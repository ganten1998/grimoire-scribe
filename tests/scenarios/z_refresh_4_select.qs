# settings: status_mode=1
# rust-only
# Issue #11, Select mode: a drag draws the selection's outline by fast (DU) frames, and its end draws every
# place it went again in grays, not only where it began and ended (a drag that went round and came back to
# its start left the page it went through dithered). DU turned the outline back to the page at each of those
# places, and the bar where it was at the first move (2.1.0: the bar goes while the box moves, and comes back
# at the lift): they count toward editor.wear, and past a quarter of the screen, once the pen has gone, one
# Clean refresh would wipe their traces (z_refresh_2_erase cleans the erasers' that way). Before 2.1.0 the bar
# went round with the box, 384 thousandths of the screen on this drag, and a Clean followed; the outline alone
# wears 57, so none does now. Delete takes the ink off by a grayscale refresh, which drives every pixel it
# changes: nothing left on the panel and nothing more to clean.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pen curve 450 700 40 60 0.6 3
pen line 350 820 750 820 40 0.6
pen leave
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
pen leave
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 3700
expect editor.select is selected
expect editor.selected is 2
expect display.presents not -1
expect editor.wear is 0
# Round the page and back to where it began: each step shown by fast frames (the pen keeps still 100 ms after
# each, past the 50 ms that has a drag show its newest place).
pen down 500 750
wait 100
pen move 700 750
wait 100
pen move 900 1000
wait 100
pen move 900 1300
wait 100
pen move 900 1600
wait 100
pen move 700 1700
wait 100
pen move 500 1600
wait 100
pen move 500 1300
wait 100
pen move 500 1000
wait 100
pen move 500 750
wait 100
# the page's gray rules where the outline went (and the bar was), drawn back by DU: dithered for now
expect display.stale.0.300.1650.2100 not 0
expect editor.wear is 58
expect display.presents not -1
pen up 500 750
wait 300
# nothing moved, and all of the way drawn again in grays
expect editor.undo_kind is add
expect display.presents is ui 10 619 1450 1174
expect display.stale.0.300.1650.2100 is 0
expect editor.wear is 58
# the pen gone: under a quarter of the screen worn, no Clean
pen leave
wait 400
expect display.presents is
expect editor.wear is 58
# Delete: one grayscale refresh, nothing left over and nothing to clean.
tap panel select.delete
wait 300
expect editor.strokes is 0
expect display.presents is ui 10 220 1822 2220
expect display.stale.0.300.1650.2100 is 0
expect editor.wear is 58
expect editor.layers is ok
