# rust-only
# The Links bar: two fingers up (or the panel's Links button, or the links action) open it along the bottom, one
# row per place the page's links go, numbered in reading order and counted: here page 3 twice, then page 2. X,
# two fingers down, the same key again and Close menu close it. The first three opens say what a tap does in a
# second header line, so the card is 60 px taller then.
# root: z_bar_0_make
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap 200 400
wait 300
expect screen is editor
expect editor.page is 1
expect editor.bar is 0
expect editor.bar.entries is 0
expect editor.bar.rect is
swipe 2 930 1300 0 -400
wait 300
expect touch.gesture is 2.up
expect editor.bar is open
expect editor.modal is links
expect editor.bar.entries is 2
expect editor.bar.1 is p3 x2
expect editor.bar.2 is p2
expect editor.bar.3 is
expect editor.bar.rect is 20 1944 1820 516
shot 01-bar-upright
# X closes it
tap 1761 2046
wait 300
expect editor.bar is 0
expect editor.modal is none
# two fingers down close it, back to the page it began on (not Pages)
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
swipe 2 930 900 0 400
wait 300
expect editor.bar is 0
expect editor.modal is none
expect editor.page is 1
# the links action opens it, and again closes it; close-menu closes it
key 37 0
wait 300
expect editor.bar is open
key 37 0
wait 300
expect editor.bar is 0
key 37 0
wait 300
expect editor.bar is open
expect editor.bar.rect is 20 2004 1820 456
key 45 0
wait 300
expect editor.bar is 0
# the panel's Links button, upright: it opens the bar, and its dot says the page has links
pentap panel links
wait 300
expect editor.bar is open
key 37 0
wait 300
expect editor.bar is 0
# sideways
rotate 1
wait 1200
swipe 2 1240 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 2
expect editor.bar.rect is 20 1384 2440 456
shot 02-bar-sideways
