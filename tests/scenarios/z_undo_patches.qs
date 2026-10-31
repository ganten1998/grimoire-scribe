# settings: status_mode=1
# settings: mirror_pdf=0
# settings: side_button=2
# settings: eraser_mode=0
# rust-only
# Undo and redo swap back the pixels each step changed (history.rs Patch): after every stroke, every
# sweep of the stroke eraser and every undo or redo, in and out of order, the page is exactly a fresh
# drawing of its strokes (live_matches_render): ink, a highlighter over it, charcoal across both, the
# stroke eraser, a circle that snapped while the pen held still, a new stroke after undos (no redo
# left), and steps undone back to an empty page.
wait 300
tap 965 95
wait 100
tap library paper.0
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen curve 300 700 40 80 0.7 4
pen line 300 900 1300 900 80 0.7 0.7
pen line 500 640 620 960 30 0.9 0.9
wait 700
expect editor.strokes is 3
expect editor.live_matches_render is 1
side on
pen line 280 880 1300 910 60 0.5 0.5
side off
wait 700
expect editor.strokes is 4
expect editor.live_matches_render is 1
pentap panel tool-picker
wait 200
pentap 1504 395
wait 200
expect editor.tool is charcoal
pen line 250 700 900 980 60 1.0 1.0
wait 700
expect editor.strokes is 5
expect editor.live_matches_render is 1
pen down 1330 1500 0.6
wait 5
pen move 1327.784 1528.158 0.6
wait 5
pen move 1321.190 1555.623 0.6
wait 5
pen move 1310.381 1581.718 0.6
wait 5
pen move 1295.623 1605.801 0.6
wait 5
pen move 1277.279 1627.279 0.6
wait 5
pen move 1255.801 1645.623 0.6
wait 5
pen move 1231.718 1660.381 0.6
wait 5
pen move 1205.623 1671.190 0.6
wait 5
pen move 1178.158 1677.784 0.6
wait 5
pen move 1150.000 1680.000 0.6
wait 5
pen move 1121.842 1677.784 0.6
wait 5
pen move 1094.377 1671.190 0.6
wait 5
pen move 1068.282 1660.381 0.6
wait 5
pen move 1044.199 1645.623 0.6
wait 5
pen move 1022.721 1627.279 0.6
wait 5
pen move 1004.377 1605.801 0.6
wait 5
pen move 989.619 1581.718 0.6
wait 5
pen move 978.810 1555.623 0.6
wait 5
pen move 972.216 1528.158 0.6
wait 5
pen move 970.000 1500.000 0.6
wait 5
pen move 972.216 1471.842 0.6
wait 5
pen move 978.810 1444.377 0.6
wait 5
pen move 989.619 1418.282 0.6
wait 5
pen move 1004.377 1394.199 0.6
wait 5
pen move 1022.721 1372.721 0.6
wait 5
pen move 1044.199 1354.377 0.6
wait 5
pen move 1068.282 1339.619 0.6
wait 5
pen move 1094.377 1328.810 0.6
wait 5
pen move 1121.842 1322.216 0.6
wait 5
pen move 1150.000 1320.000 0.6
wait 5
pen move 1178.158 1322.216 0.6
wait 5
pen move 1205.623 1328.810 0.6
wait 5
pen move 1231.718 1339.619 0.6
wait 5
pen move 1255.801 1354.377 0.6
wait 5
pen move 1277.279 1372.721 0.6
wait 5
pen move 1295.623 1394.199 0.6
wait 5
pen move 1310.381 1418.282 0.6
wait 5
pen move 1321.190 1444.377 0.6
wait 5
pen move 1327.784 1471.842 0.6
wait 5
pen move 1330.000 1500.000 0.6
wait 900
pen up 1330 1500
wait 700
expect editor.strokes is 6
expect editor.live_matches_render is 1
# The stroke eraser through the first strokes
eraser line 560 600 600 1000 20
wait 700
expect editor.live_matches_render is 1
# Undo the sweep, the circle and the charcoal; redo two; undo three again
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
expect editor.strokes is 4
expect editor.undo_patched is 3
pentap panel redo
wait 300
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.live_matches_render is 1
expect editor.strokes is 6
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
# A new stroke drops the redo steps; undo it and the three first strokes, then redo all four
pen line 400 1800 1400 1850 40 0.8 0.8
wait 700
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.live_matches_render is 1
expect editor.strokes is 0
pentap panel redo
wait 300
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.live_matches_render is 1
expect editor.strokes is 4
expect editor.undo_patched is 16
quit
