# settings: status_mode=1
# The stroke eraser (eraser_mode=0) takes a stroke away only where it touches ink of it that shows:
# over the part of a pencil line that white ink took away (the page there is its paper again) it
# takes nothing, and the line stays; over the part that shows it takes the line, and undo brings it
# back. Ink written after the white over that part is another stroke: the eraser there takes it and
# leaves the line. (A broad white marker without grain, and a small eraser, so the white covers the
# eraser's whole reach.)
# settings: mirror_pdf=0
# settings: marker.size_min_mm=8
# settings: marker.size_max_mm=8
# settings: marker.opacity_min=1
# settings: marker.grain=0
# settings: eraser.size_min_mm=1
# settings: eraser.size_max_mm=1
# rust-only
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
pentap panel tool-picker
wait 200
pentap 1425 878
wait 200
pentap panel picker.tool:marker
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
pentap panel undo
wait 300
expect editor.strokes is 2
expect editor.live_matches_render is 1
# The eraser button (the tip as the stroke eraser) is the same: over the white, nothing
pentap panel eraser
wait 100
expect editor.tool is eraser
pen line 896 950 904 1050 12 0.6 0.6
wait 700
expect editor.strokes is 2
pen line 1196 950 1204 1050 12 0.6 0.6
wait 700
expect editor.strokes is 1
# The line back; the eraser tapped again: back to the marker (white). Black pencil then, down
# through the whited part after the white (a correction written over a word whited out)
pentap panel undo
wait 300
expect editor.strokes is 2
pentap panel eraser
wait 100
expect editor.tool is marker
pentap panel tool-picker
wait 200
pentap 951 878
wait 200
expect settings.white is 0
pentap panel picker.tool:pencil
wait 200
expect editor.modal is none
expect editor.tool is pencil
pen line 900 850 900 1150 40 0.7 0.7
wait 700
expect editor.strokes is 3
expect editor.erase_strokes is 1
shot 03-pencil-over-the-whited-part
# The eraser end where the new pencil crosses the line's whited part: the pencil goes, the line
# (whose ink there white took away) stays, and so does its ink either side
eraser line 897 995 903 1005 6
wait 700
shot 04-the-new-pencil-taken
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.stroke_look is white marker
expect editor.live_matches_render is 1
# Undone, the pencil is back; the eraser on the line where it shows takes the line
pentap panel undo
wait 300
expect editor.strokes is 3
eraser line 596 950 604 1050 12
wait 700
expect editor.strokes is 2
expect editor.stroke_look is pencil
expect editor.live_matches_render is 1
pentap panel library
wait 500
expect screen is library
quit
