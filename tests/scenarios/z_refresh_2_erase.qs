# settings: status_mode=1
# rust-only
# Issue #11, erasing: the eraser's fast (DU) frames turn ink back to paper and leave a trace of it that the
# grayscale refresh after them cannot reach. editor.wear counts what it took (thousandths of the screen, each
# stroke as the 4 mm band its line lies in); a word stays far below a quarter of the screen and never makes a
# flash. Past a quarter, once the pen has gone, one Clean refresh (a flash of that place alone) wipes the
# traces and the count starts again. Nothing at all while the pen is still near.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
# a word, and a scribble that covers most of a page's width
pen curve 300 400 40 60 0.6 3
pen leave
pen down 200 700 0.6
pen move 1500 740 0.6
pen move 200 780 0.6
pen move 1500 820 0.6
pen move 200 860 0.6
pen move 1500 900 0.6
pen move 200 940 0.6
pen move 1500 980 0.6
pen move 200 1020 0.6
pen move 1500 1060 0.6
pen move 200 1100 0.6
pen move 1500 1140 0.6
pen move 200 1180 0.6
pen move 1500 1220 0.6
pen move 200 1260 0.6
pen move 1500 1300 0.6
pen move 200 1340 0.6
pen move 1500 1380 0.6
pen move 200 1420 0.6
pen move 1500 1460 0.6
pen move 200 1500 0.6
pen move 1500 1540 0.6
pen move 200 1580 0.6
pen move 1500 1620 0.6
pen move 200 1660 0.6
pen up 200 1660
pen leave
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 3700
expect editor.strokes is 2
expect editor.wear is 0
expect display.presents not -1
# The word erased: under a hundredth of the screen, the grayscale refresh alone, before and after the pen goes.
eraser line 250 330 350 470 10
expect editor.strokes is 1
expect editor.wear is 8
pen leave
wait 400
expect display.presents is ui 270 220 1562 1734
expect editor.wear is 8
# The scribble erased: past a quarter, but the pen is still near, so no flash yet.
eraser line 800 650 820 1750 20
expect editor.strokes is 0
expect editor.wear is 288
wait 400
expect display.presents not has clean
# The pen gone: one clean where the erasing was (the word's place and the scribble's), and the count starts again.
pen leave
wait 400
expect display.presents is clean 190 350 1317 1317
expect editor.wear is 0
expect display.stale.0.300.1650.2000 is 0
expect editor.layers is ok
# Undo brings the ink back by grayscale refreshes; nothing more to clean.
tap panel undo
wait 200
tap panel undo
wait 400
expect editor.strokes is 2
expect editor.wear is 0
expect display.presents not has clean
expect display.stale.0.300.1650.2000 is 0
