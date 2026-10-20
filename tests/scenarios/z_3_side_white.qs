# The pen button drawing white (side_button=3): while it is held, the pen draws white with its own
# drawing tool, over ink the tip drew; with the eraser picked, with the drawing tool the toolbar
# shows. The tip with the eraser picked is the stroke eraser here (eraser_mode=0): it takes whole
# ink strokes away and leaves the white ones, which would bring back what they cover.
# settings: side_button=3
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect settings.side_button is 3
expect settings.eraser_mode is 0
pen curve 300 700 40 80 0.7 4
pen line 300 900 1300 900 80 0.7 0.7
wait 300
expect editor.strokes is 2
# Held: white pencil through the writing and the line
side on
pen line 500 640 620 960 30 0.9 0.9
side off
wait 700
shot 01-button-white-pencil
expect editor.strokes is 3
expect editor.erase_strokes is 1
expect editor.tool is pencil
expect editor.stroke_look is white pencil
expect editor.stroke_settings is pencil
expect editor.live_matches_render is 1
# The marker picked: the button draws white marker
pentap 1768 358
wait 200
pentap 1504 561
wait 200
expect editor.tool is marker
pen line 300 1100 1300 1150 80 0.8 0.8
side on
pen line 700 1050 760 1200 20 0.9 0.9
side off
wait 700
shot 02-button-white-marker
expect editor.strokes is 5
expect editor.erase_strokes is 2
expect editor.stroke_look is white marker
expect editor.stroke_settings is marker
expect editor.live_matches_render is 1
# The eraser picked: the button still draws white with the toolbar's tool (the marker)
pentap 1768 466
wait 100
expect editor.tool is eraser
side on
pen line 1000 1050 1060 1200 20 0.9 0.9
side off
wait 300
expect editor.strokes is 6
expect editor.erase_strokes is 3
expect editor.stroke_look is white marker
expect editor.stroke_settings is marker
expect editor.live_matches_render is 1
# The tip: the stroke eraser takes the pencil line away, not the white pencil across it
pen line 400 860 420 940 10 0.6 0.6
wait 700
shot 03-stroke-eraser-leaves-white
expect editor.strokes is 5
expect editor.erase_strokes is 3
expect editor.live_matches_render is 1
pentap 1768 594
wait 300
expect editor.strokes is 6
expect editor.live_matches_render is 1
pentap 1768 1566
wait 1500
expect screen is library
expect bg.busy is 0
quit
