# rust-only
# Default status bar reserves the notebook viewport, survives pen input and rotation,
# and never appears on the Library or an open sheet.
# env: TZ=UTC
wait 500
clock 1700000000
battery 87
expect settings.status_mode is 0
expect status.shown is 0
tap 965 95
wait 100
tap 1564 2254
wait 500
expect screen is editor
expect status.shown is 1
expect status.rect is 0 0 1860 104
expect status.text has 1 / 1
expect status.text has From 0
expect status.text has To 0
expect editor.scale is 0.96
expect editor.zoomed is 0
shot 01-upright
# A pen tap in the strip must not deposit any ink.
pen down 300 30 0.6
pen up 300 30
wait 100
expect editor.strokes is 0
rotate 1
wait 1200
expect status.rect is 0 0 2480 104
expect status.text has 1 / 1
expect editor.zoomed is 0
shot 02-sideways
pentap panel next
wait 500
expect status.text has 2 / 2
shot 03-new-page
quit
