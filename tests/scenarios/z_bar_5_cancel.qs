# rust-only
# Cancelling a peek: X returns to the page the bar began on with one Ui, or one Full after three peeks (GL16
# ghosting builds up); the back chip returns to it with the bar kept.
# root: z_bar_0_make
wait 3700
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
tap 1381 2218
wait 300
tap 479 2218
wait 300
expect editor.bar.peek is 3
expect display.presents not -1
tap 1761 2046
wait 300
expect editor.bar is 0
expect editor.page is 1
expect display.presents is fast 1702 1996 118 100, full 0 0 1860 2480
# one peek then X: a Ui, no flash
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
expect display.presents not -1
tap 1761 2046
wait 300
expect editor.bar is 0
expect editor.page is 1
expect display.presents is fast 1702 1996 118 100, ui 0 0 1860 2480
# the back chip: the origin again, the bar stays
swipe 2 930 1300 0 -400
wait 300
tap 1381 2218
wait 300
expect editor.bar.peek is 2
tap 150 2046
wait 300
expect editor.bar is open
expect editor.bar.peek is 0
expect editor.page is 1
shot 01-back-chip
