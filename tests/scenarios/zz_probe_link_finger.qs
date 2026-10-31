# settings: status_mode=1
# rust-only
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# settings: gesture_new_page=0
# copy: tests/fixtures/link-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
wait 100
pentap panel next
wait 100
pentap panel previous
wait 100
pentap panel previous
wait 100
expect editor.page is 1
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 1
# D: a lasso held to snap a rectangle
key 38 0
pen down 300 1700
pen move 900 1700
pen move 900 2000
pen move 300 2000
pen move 305 1710
wait 900
pen up 305 1710
wait 100
shot f1-up
pen leave
wait 200
shot f2-leave
expect editor.ui_at.930.549 is linkto.pages
tap 930 549
wait 200
shot e1-pages
tap 703 340
wait 200
shot e2-picked
expect editor.links is 1 1
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 1
tap 600 1850
wait 300
expect editor.page is 2
tap panel previous
wait 300
# B: lasso, then a finger tap outside Link to... cancels it
key 38 0
pen line 300 1000 900 1200 30 0.6 0.6
pen leave
wait 200
tap 600 1800
wait 200
expect toast is No link made.
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
quit
