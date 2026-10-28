# settings: status_mode=1
# rust-only
# The pen outside the bar while it looks at a page: that page stays (at the pen's down, with its Full), and the
# stroke the pen began is swallowed: nothing is drawn, no ink is marked live.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
expect editor.bar is peek
pen down 300 600 0.5
expect editor.bar is 0
expect editor.page is 3
expect display.ink is 0
pen move 600 610 0.7
pen move 900 620 0.9
expect display.ink is 0
pen up 900 620
wait 300
expect editor.strokes is 0
expect editor.page is 3
expect display.ink is 0
# and the next stroke writes
pen line 300 700 900 720 30 0.5 0.9
expect editor.strokes is 1
