# The stroke eraser (eraser_mode=0) takes a stroke away only where it touches ink of it that shows:
# over the part of a pencil line that white ink took away (the page there is its paper again) it
# takes nothing, and the line stays; over the part that shows it takes the line, and undo brings it
# back. (A broad white marker without grain, and a small eraser, so the white covers the eraser's
# whole reach.)
# settings: mirror_pdf=0
# settings: marker.size_min_mm=8
# settings: marker.size_max_mm=8
# settings: marker.opacity_min=1
# settings: marker.grain=0
# settings: eraser.size_min_mm=1
# settings: eraser.size_max_mm=1
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect settings.eraser_mode is 0
pen line 300 1000 1500 1000 60 0.7 0.7
wait 300
expect editor.strokes is 1
# White marker across the line
pentap 1768 358
wait 200
pentap 1425 878
wait 200
pentap 1504 561
wait 200
expect editor.modal is none
expect settings.white is 1
expect editor.tool is marker
pen line 900 800 900 1200 40 1.0 1.0
wait 700
shot 01-line-whited-out-in-the-middle
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
# The eraser end over the whited-out part: nothing to take, the line stays
eraser line 896 950 904 1050 12
wait 700
expect editor.strokes is 2
expect editor.erase_strokes is 1
# Over the part that shows: the line goes (the white marker stays: erase strokes do)
eraser line 596 950 604 1050 12
wait 700
shot 02-line-taken-where-it-shows
expect editor.strokes is 1
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
pentap 1768 594
wait 300
expect editor.strokes is 2
expect editor.live_matches_render is 1
# The eraser button (the tip as the stroke eraser) is the same: over the white, nothing
pentap 1768 466
wait 100
expect editor.tool is eraser
pen line 896 950 904 1050 12 0.6 0.6
wait 700
expect editor.strokes is 2
pen line 1196 950 1204 1050 12 0.6 0.6
wait 700
expect editor.strokes is 1
pentap 1768 1566
wait 500
expect screen is library
quit
