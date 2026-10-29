# rust-only
# Making a link leaves no ink (PHASE2 6.4): while the lasso is drawn the driver is never asked for its pen mode and
# no frame is marked live ink; after the lift no stroke was added, the last fast refresh was not ink, and the page
# canvas is what the page renders. With Live view on, the same lasso adds no stroke either.
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
expect editor.page is 1
expect editor.strokes is 0
expect editor.links is 3 3
key 38 0
wait 60
expect editor.strip is new
expect editor.link_mode is 1
expect editor.lasso is 0
expect editor.ui_at.1695.2395 is strip.cancel
expect editor.ui_at.900.2395 is
expect editor.ui_at.900.900 is
pen down 1000 600 0.6
pen move 1100 650 0.6
pen move 1300 800 0.6
expect display.pen_mode is 0
expect display.ink is 0
expect editor.lasso not 0
expect editor.strokes is 0
pen move 1500 700 0.6
pen up 1500 700
wait 100
expect display.last_fast not ink
expect display.pen_mode is 0
expect editor.strokes is 0
expect editor.strip is 0
expect editor.lasso is 0
expect editor.modal is pages
expect editor.picking is new
expect editor.link_area is 1000 600 500 200
expect editor.live_matches_render is 1
# Back: no link, and still no ink
tap 125 95
wait 100
expect toast is No link made.
expect editor.link_area is
expect editor.links is 3 3
expect editor.strokes is 0
# The strip's Cancel, by finger
key 38 0
wait 60
expect editor.strip is new
tap 1695 2395
wait 60
expect editor.strip is 0
expect toast is No link made.
pentap panel library
waitbg
# Live view on, then the same lasso
tap 1345 95
wait 300
tap 318 2100
wait 300
expect live.on is 1
tap 1538 2100
wait 300
tap 200 400
wait 300
expect screen is editor
key 38 0
wait 60
pen down 1000 600 0.6
pen move 1100 650 0.6
pen move 1300 800 0.6
expect display.ink is 0
expect display.pen_mode is 0
pen move 1500 700 0.6
pen up 1500 700
wait 100
expect editor.strokes is 0
expect display.last_fast not ink
expect editor.modal is pages
tap 125 95
wait 100
expect editor.strokes is 0
expect editor.links is 3 3
quit
