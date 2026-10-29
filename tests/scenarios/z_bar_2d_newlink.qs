# settings: status_mode=1
# rust-only
# New link on the Links bar: the bar closes and the strip comes up (the next pen stroke is the lasso around the
# link's area, no ink, then the Pages view picks its page); with another page looked at, it is that page the link
# is made on (the bar stays on it first). Opening the bar while the strip is up disarms it, silently.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
tap 705 2370
wait 300
expect editor.bar is 0
expect editor.link_mode is 1
expect editor.page is 1
expect editor.strip is new
# the bar again: the waiting link is not made, and nothing says so
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.link_mode is 0
expect editor.strip is 0
# a page looked at: New link stays on it first
tap 1381 2218
wait 300
expect editor.bar is peek
tap 705 2370
wait 300
expect editor.bar is 0
expect editor.page is 2
expect editor.link_mode is 1
expect editor.back is 1 1
