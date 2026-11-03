# settings: status_mode=1
# settings: eraser_mode=1
# rust-only
# Issue #11, the precise eraser: its erase strokes are drawn by fast (DU) frames as ink is, but they turn ink
# back to paper, which leaves a trace a grayscale refresh cannot reach. Each kept one counts what it swept
# (its length times its tip, at least 4 mm wide, within its box) toward editor.wear at its pen-up; past a
# quarter of the screen, at the next pause (here with the pen still near: 1.5 s after the stroke, as the
# grayscale settle), one Clean refresh wipes them. A short erase never does, and ink written with the pen
# adds nothing.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
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
wait 3700
expect editor.strokes is 1
expect editor.wear is 0
expect display.presents not -1
# a short erase across one line of the scribble
eraser line 700 720 700 800 8
pen leave
wait 400
expect editor.strokes is 2
expect editor.wear is 2
expect display.presents not has clean
# a scribble of the eraser over most of it: no flash with the pen near, then one where it swept
eraser down 200 690 0.6
eraser move 1500 730 0.6
eraser move 200 770 0.6
eraser move 1500 810 0.6
eraser move 200 850 0.6
eraser move 1500 890 0.6
eraser move 200 930 0.6
eraser move 1500 970 0.6
eraser move 200 1010 0.6
eraser move 1500 1050 0.6
eraser move 200 1090 0.6
eraser move 1500 1130 0.6
eraser move 200 1170 0.6
eraser move 1500 1210 0.6
eraser move 200 1250 0.6
eraser move 1500 1290 0.6
eraser move 200 1330 0.6
eraser move 1500 1370 0.6
eraser move 200 1410 0.6
eraser move 1500 1450 0.6
eraser move 200 1490 0.6
eraser move 1500 1530 0.6
eraser move 200 1570 0.6
eraser move 1500 1610 0.6
eraser move 200 1650 0.6
eraser up 200 1650
expect editor.strokes is 3
expect editor.wear is 318
wait 400
expect display.presents not has clean
# (the pen stays near: the clean waits as the grayscale settle does, 1.5 s after the stroke, and goes out
# with it as one refresh)
wait 1300
expect display.presents is clean 154 644 1394 1054
expect editor.wear is 0
expect display.stale.0.300.1650.2000 is 0
expect editor.layers is ok
