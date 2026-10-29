# rust-only
# Legacy Bar preferences migrate to the small enabled top-left corner, without reserving page space.
# settings: status_mode=0
# settings: status_on=0
# settings: status_size=2
# settings: status_corner=3
# env: TZ=UTC
wait 500
clock 1700000000
battery 87
expect settings.status_mode is 1
expect settings.status_on is 1
expect settings.status_size is 0
expect settings.status_corner is 0
expect status.shown is 0
tap 965 95
wait 100
tap 1564 2254
wait 500
expect screen is editor
expect status.shown is 1
expect status.reserved is 0
expect status.text has 87%
expect editor.scale is 1.00
shot 01-default-corner
# The old strip is page space again.
pen down 300 30 0.6
pen up 300 30
pen leave
wait 100
expect editor.strokes is 1
rotate 1
wait 1200
expect status.reserved is 0
expect status.shown is 1
expect editor.zoomed is 0
shot 02-sideways
pentap panel next
wait 500
expect status.shown is 1
expect status.text has 87%
quit
