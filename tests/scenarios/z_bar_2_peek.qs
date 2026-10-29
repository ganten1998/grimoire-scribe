# settings: status_mode=1
# rust-only
# Looking at where a link goes without going there: tap an entry in the Links bar and the page above it becomes
# that page, one GL16 of the page area and the bar together (no flash), the open page, its history and its
# link boxes untouched (notebook.last_page stays the origin's); another entry shows that one; a finger tap on the
# page above stays there, with one Full, and does nothing else (the link under it is not followed); Back to the
# page before returns, and a back list of one is left behind.
# root: z_bar_0_make
wait 3700
tap 200 400
wait 300
expect screen is editor
expect editor.page is 1
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect display.presents not -1
tap 479 2218
wait 300
expect editor.bar is peek
expect editor.bar.peek is 3
expect editor.page is 1
expect editor.zoomed is 0
expect notebook.last_page is 1
expect display.presents is fast 40 2156 878 124, ui 0 0 1860 2460
shot 01-peek-page-3
tap 1381 2218
wait 300
expect editor.bar is peek
expect editor.bar.peek is 2
expect editor.page is 1
expect editor.back is 0 0
shot 02-peek-page-2
# a finger on the page above the bar (on the link area there): it stays on page 2, the link not followed
expect display.presents not -1
tap 600 1100
wait 300
expect editor.bar is 0
expect editor.bar.peek is 0
expect editor.page is 2
expect display.presents is full 0 0 1860 2480
expect notebook.last_page is 2
expect editor.back is 1 1
# Back to the page before
key 48 0
wait 300
expect editor.page is 1
expect editor.back is 0 0
# a finger resting on a link while the pen comes near follows nothing
finger 1 down 600 1100
frame
pen hover 900 2000
finger 1 up 600 1100
frame
wait 200
expect editor.page is 1
pen leave
# the long hold on a link on the page: its border and the long-press card (PHASE2 8), over the page; a tap
# outside closes it
touch down 600 1100
wait 800
expect editor.link_card is 1
touch up 600 1100
expect editor.modal is card
tap 1200 1900
wait 300
expect editor.modal is none
expect editor.page is 1
