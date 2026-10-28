# settings: shortcuts_enabled=1
# copy: tests/fixtures/status-keys .
# env: TZ=UTC
# rust-only
# The clock and battery in each corner of the notebook page (settings.ini status_corner: 0 top left, the
# default, 1 top right, 2 bottom right, 3 bottom left): the Display sheet's corner cell and the status-corner
# shortcut (the C key and the East button here) step it clockwise; its tap zone, a 14 mm square, follows it; moving
# it is two small refreshes, the new place and the old, never one across the screen; sideways it is the turned
# screen's corner. The box reads left to right in every corner: the time, the battery, its percent.
wait 3800
clock 1700000000
battery 87
wait 100
expect settings.status_corner is 0
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect status.zone is 0 0 165 165
# Off, only the chosen corner takes a tap: the others are the page's.
tap 1790 70
wait 150
tap 1790 2410
wait 150
tap 70 2410
wait 150
expect settings.status_on is 0
tap 70 70
wait 300
expect settings.status_on is 1
expect status.rect is 6 6 331 48
shot 01-top-left
# The Display sheet's corner cell, beside the size and Done: top right.
tap panel display
wait 400
expect editor.modal is display
tap 997 1860
wait 300
expect settings.status_corner is 1
expect status.zone is 1695 0 165 165
tap 1460 1860
wait 400
expect editor.modal is none
expect status.shown is 1
expect status.rect is 1523 6 331 48
expect status.icon is 1735 16 54 27
shot 02-top-right
# The zone moved with it: the old corner is the page's now, the new one the clock's.
tap 70 70
wait 150
expect settings.status_on is 1
tap 1790 70
wait 300
expect settings.status_on is 0
expect status.shown is 0
tap 1790 70
wait 300
expect settings.status_on is 1
# The shortcut: the next corner, with the box going from one place to the other in two small refreshes.
expect display.presents not -1
key 46 0
wait 300
expect settings.status_corner is 2
expect status.zone is 1695 2315 165 165
expect status.rect is 1523 2426 331 48
expect display.presents is ui 1523 6 331 48, ui 1523 2426 331 48
shot 03-bottom-right
key 305 0 1
wait 300
expect settings.status_corner is 3
expect status.zone is 0 2315 165 165
expect status.rect is 6 2426 331 48
expect display.presents is ui 1523 2426 331 48, ui 6 2426 331 48
shot 04-bottom-left
# A tap in the zone toggles there; the box is as a tap on it too.
tap 70 2410
wait 300
expect settings.status_on is 0
tap 70 2410
wait 300
expect settings.status_on is 1
tap 250 2440
wait 300
expect settings.status_on is 0
tap 70 2410
wait 300
key 46 0
wait 300
expect settings.status_corner is 0
expect status.rect is 6 6 331 48
# The Library shows none of it, and the shortcut there says so, with a toast of its own.
tap panel library
wait 600
expect screen is library
expect status.shown is 0
key 46 0
wait 300
expect settings.status_corner is 1
expect toast is Clock and battery: top right corner of the page
key 46 0
wait 300
expect toast is Clock and battery: bottom right corner of the page
key 46 0
wait 300
expect toast is Clock and battery: bottom left corner of the page
key 46 0
wait 300
expect toast is Clock and battery: top left corner of the page
expect settings.status_corner is 0
# Sideways: the corners of the turned screen (2480 by 1860).
key 46 0
wait 300
key 46 0
wait 300
expect settings.status_corner is 2
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
rotate 1
wait 500
expect rotation is 1
expect status.zone is 2315 1695 165 165
expect status.shown is 1
expect status.rect is 2143 1806 331 48
shot 05-sideways-bottom-right
tap 2400 1780
wait 300
expect settings.status_on is 0
tap 2400 1780
wait 300
expect settings.status_on is 1
quit
