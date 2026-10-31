# settings: status_size=0
# settings: status_mode=1
# settings: status_on=0
# env: TZ=UTC
# rust-only
# The battery in the clock and battery corner (status.rs): pixel art left of the percent, an outline with a terminal
# and a bar of up to 12 lit columns that is the charge (the nearest share, one column for any charge at all), a
# bolt over it while it charges, no battery at all where the kernel has no reading. It scales with the size setting:
# its unit is a tenth of the text's height (3, 4 and 6 px: 54 by 27, 72 by 36 and 108 by 54).
wait 3800
clock 1700000000
battery 100
wait 100
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
tap 70 70
wait 300
expect status.shown is 1
expect status.fill is 12
expect status.icon is 218 16 54 27
expect status.charging is 0
shot 01-100
battery 87
wait 300
expect status.fill is 10
expect status.text is Nov 14, 22:13  87%
battery 50
wait 300
expect status.fill is 6
shot 02-50
battery 30
wait 300
expect status.fill is 4
battery 12
wait 300
expect status.fill is 1
shot 03-12
battery 4
wait 300
expect status.fill is 1
battery 0
wait 300
expect status.fill is 0
expect status.text is Nov 14, 22:13  0%
shot 04-0
# Charging: the bolt, over any charge, with the box and the battery where they were.
battery 0 charging
wait 300
expect status.charging is 1
expect status.fill is 0
expect status.rect is 6 6 318 48
shot 05-0-charging
battery 50 charging
wait 300
expect status.fill is 6
expect status.icon is 218 16 54 27
shot 06-50-charging
battery 100 charging
wait 300
expect status.fill is 12
shot 07-100-charging
# No reading: the time alone, no battery.
battery none
wait 300
expect status.icon is
expect status.fill is
expect status.charging is 0
expect status.text is Nov 14, 22:13
shot 08-none
# The art scales with the size: Display's size cell, small to medium to large.
battery 50 charging
wait 300
tap panel display
wait 400
tap 472 1860
wait 300
tap 1460 1860
wait 400
expect settings.status_size is 1
expect status.icon is 302 21 72 36
shot 09-medium-50-charging
tap panel display
wait 400
tap 472 1860
wait 300
tap 1460 1860
wait 400
expect settings.status_size is 2
expect status.icon is 401 23 108 54
shot 10-large-50-charging
battery 87
wait 300
shot 11-large-87
quit
