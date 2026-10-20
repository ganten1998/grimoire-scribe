# The precise eraser keeps its size on the screen (editor.stroke_screen_mm: the newest stroke's
# width at the zoom it shows at): zoomed in, its widths on the page are divided by the zoom;
# sideways too. Live equals a fresh render through all of it.
# settings: eraser_mode=1
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
pen curve 300 520 40 70 0.7 5
pen line 300 640 1300 660 60 0.7 0.7
wait 200
expect editor.strokes is 2
eraser line 400 480 440 700 12
wait 300
expect editor.strokes is 3
expect editor.stroke_screen_mm is 7.00
expect editor.live_matches_render is 1
# Zoomed in around the writing: as big on the screen, smaller on the page
pinch 600 600 300 750 6
wait 700
shot 01-zoomed
expect editor.zoomed is 1
eraser line 900 300 1000 1100 30
wait 700
shot 02-zoomed-erased
expect editor.strokes is 4
expect editor.stroke_screen_mm is 7.00
expect editor.live_matches_render is 1
tap 930 74
wait 300
expect editor.zoom is 1.00
shot 03-zoom-reset
expect editor.live_matches_render is 1
# Sideways
rotate 1
wait 600
expect rotation is 1
expect editor.zoom is 1.00
eraser line 700 500 760 1000 20
wait 700
shot 04-sideways-erased
expect editor.strokes is 5
expect editor.stroke_screen_mm is 7.00
expect editor.live_matches_render is 1
rotate 0
wait 600
expect rotation is 0
expect editor.live_matches_render is 1
pentap 1768 1566
wait 1500
expect screen is library
expect bg.busy is 0
quit
