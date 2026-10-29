# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
wait 200
tap 1535 95
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.editing
wait 100
expect library.gestures_group is editing
tap library gestures.row.1
wait 100
expect library.gestures.learning is undo
expect library.record is wait
swipe 2 930 1100 0 400 10 295 300
expect library.record.line is Put the fingers down together.
swipe 2 930 1100 400 400
expect library.record.line is Swipe in a straight line.
tap library record.retry
wait 100
expect library.record is wait
finger 1 down 750 900
finger 2 down 1050 900
frame 30
finger 1 move 715 940
finger 2 move 1085 940
frame 20
finger 1 move 680 980
finger 2 move 1120 980
frame 20
finger 1 move 645 1020
finger 2 move 1155 1020
frame 20
finger 1 move 610 1060
finger 2 move 1190 1060
frame 20
finger 1 move 575 1100
finger 2 move 1225 1100
frame 20
finger 1 move 540 1140
finger 2 move 1260 1140
frame 20
finger 1 move 505 1180
finger 2 move 1295 1180
frame 20
finger 1 move 470 1220
finger 2 move 1330 1220
frame 20
finger 1 move 435 1260
finger 2 move 1365 1260
frame 20
finger 1 move 400 1300
finger 2 move 1400 1300
frame 20
finger 1 up 400 1300
finger 2 up 1400 1300
frame
expect library.record.line is Two fingers spreading is kept for zoom.
tap library record.retry
wait 100
expect library.record is wait
tapn 1 930 1100
expect library.record.line is One finger is kept for taps and links.
pen line 500 1100 600 1100 5 0.6 0.6
pen leave
wait 300
swipe 2 930 1100 0 400
expect library.record.line is The pen is near: move it away.
