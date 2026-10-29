# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# root: s_shape_1_record
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pen line 350 650 1250 680 12 0.6 0.6
pen leave
wait 1100
expect editor.strokes is 1
expect display.presents not clear
shape Z 930 1150 45 400
expect editor.strokes is 0
expect fshape.fired is 1
expect fshape.last has fired undo
expect toast is Undo
expect display.presents is fast 657 877 519 43, fast 847 905 329 329, fast 691 1219 171 205, fast 793 1409 281 15, fast 1059 1409 132 15, ui 334 220 1498 2140
# Zoom and rotation use the same learned geometry.
pen line 350 650 1250 680 12 0.6 0.6
pen leave
wait 1100
pinch 930 1200 300 600 6
wait 1100
shape Z 930 1150 45 600
expect editor.strokes is 0
expect fshape.fired is 1
rotate 1
wait 1200
pen line 350 650 1250 680 12 0.6 0.6
pen leave
wait 1100
shape Z 1100 950 45 600
expect editor.strokes is 0
expect fshape.fired is 1
