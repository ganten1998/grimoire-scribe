# settings: status_mode=1
# Resting fingers / a hand must not zoom or flash; a real pinch still zooms (with a GL16 settle)
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen curve 300 520 40 60 0.7 4
wait 200
expect editor.strokes is 1
expect editor.zoom is 1.00
pinch 900 1400 400 420 6 10 15
wait 300
shot 01-resting-fingers-no-zoom
expect editor.zoom is 1.00
pinch 900 900 300 700 6
wait 300
shot 02-real-pinch-zooms
expect editor.zoom not 1.00
expect editor.strokes is 1
quit
