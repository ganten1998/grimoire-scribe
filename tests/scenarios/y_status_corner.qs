# settings: shortcuts_enabled=1
# copy: tests/fixtures/status-keys .
# env: TZ=UTC
# The clock and battery corner (status.rs, the Rust build only): a finger tap in the top left corner,
# a 12 mm square, on a screen with no control there turns a small time, battery percent and bolt (while
# charging) on and off; the status-toggle shortcut does the same on any screen; the Display sheet sets
# its size. It is an overlay: a small GL16 of its own rect at each minute and when the battery or its
# charging changes, held back while the pen is near or a stroke is drawn, and never in the sleep screen.
# The script sets the clock (2023-11-14 22:13:20 UTC) and the battery.
# (The first 4 s are the startup's two unconditional repaints, which this checks the refreshes of.)
wait 3800
clock 1700000000
battery 87
wait 100
expect settings.status_on is 0
expect settings.status_size is 0
expect status.shown is 0
expect status.text is
expect status.rect is
expect status.zone is 0 0 142 142
# The Library, on: its own small refresh, nothing else.
expect display.presents not -1
tap 70 70
wait 300
expect settings.status_on is 1
expect status.shown is 1
expect status.text is Nov 14, 22:13  87%
expect status.charging is 0
expect status.rect is 6 6 278 48
expect display.presents is ui 6 6 278 48
shot 01-library-on
# A tap on the indicator itself turns it off (its box is part of the corner), and the refresh takes it
# down where it was.
expect status.zone is 0 0 284 142
tap 250 30
wait 300
expect settings.status_on is 0
expect status.shown is 0
expect display.presents is ui 6 6 278 48
tap 70 70
wait 300
expect status.shown is 1
expect display.presents is ui 6 6 278 48
# Not a tap: a finger that stays down, one that swipes out of the corner, one that comes down outside it,
# the pen.
touch down 70 70
wait 700
touch up 70 70
wait 200
expect settings.status_on is 1
drag touch 70 70 500 70 4
wait 200
expect settings.status_on is 1
tap 200 150
wait 200
expect settings.status_on is 1
pentap 70 70
pen leave
wait 200
expect settings.status_on is 1
expect display.presents not -1
# A control under the finger keeps it: Pen sensitivity's Back is in the corner, and a sheet that is up
# takes every tap.
tap 1535 95
wait 200
expect library.sheet is settings
tap 70 70
wait 200
expect library.sheet is settings
expect settings.status_on is 1
tap 930 446
wait 400
expect screen is penset
tap 100 100
wait 400
expect screen is library
expect settings.status_on is 1
expect status.shown is 1
# The size, in the Display sheet beside Done: small, medium, large, small again. Each is one refresh
# of the indicator's new box (and of what it covered).
tap 1535 95
wait 200
tap 930 780
wait 200
expect library.sheet is display
expect display.presents not -1
tap 735 1533
wait 300
expect settings.status_size is 1
expect status.rect is 6 6 389 66
expect display.presents is ui 180 419 1500 1222, ui 6 6 389 66
tap 735 1533
wait 300
expect settings.status_size is 2
expect status.rect is 6 6 518 88
tap 735 1533
wait 300
expect settings.status_size is 0
expect status.rect is 6 6 278 48
tap 735 1533
wait 300
tap 735 1533
wait 300
expect settings.status_size is 2
shot 02-display-sheet-large
tap 1460 1533
wait 300
expect library.sheet is settings
# Charging: the bolt widens the box; a battery that changes is one refresh, a minute passing another,
# and neither a second.
expect display.presents not -1
battery 86 charging
wait 300
expect status.text is Nov 14, 22:13  86%
expect status.charging is 1
expect status.rect is 6 6 567 88
expect display.presents is ui 6 6 567 88
clock 1700000030
wait 300
expect display.presents is
clock 1700000060
wait 300
expect status.text is Nov 14, 22:14  86%
expect display.presents is ui 6 6 567 88
battery none
wait 300
expect status.text is Nov 14, 22:14
expect status.charging is 0
expect display.presents is ui 6 6 567 88
battery 100
wait 300
expect status.text is Nov 14, 22:14  100%
shot 03-large-100
tap 1450 1930
wait 300
expect library.sheet is none
# The size is kept with the setting.
expect settings.status_size is 2
# The editor: a notebook, the corner on a page.
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect status.shown is 1
expect status.text is Nov 14, 22:14  100%
shot 04-editor
tap 70 70
wait 300
expect settings.status_on is 0
expect status.shown is 0
tap 70 70
wait 300
expect settings.status_on is 1
expect status.shown is 1
# The pen near: nothing is sent, the clock waits; when it leaves the refresh comes.
expect display.presents not -1
pen hover 900 900
battery 60
clock 1700000100
wait 300
expect display.presents is
expect status.text is Nov 14, 22:14  100%
pen leave
wait 300
expect status.text is Nov 14, 22:15  60%
# (one refresh, over the wider box that 100% had and the narrower one that 60% has)
expect display.presents is ui 6 6 544 88
# A stroke in progress: the same, and the stroke's own refreshes are not held for it.
pen down 400 600 0.6
pen move 460 640 0.6
battery 59
wait 300
expect status.text is Nov 14, 22:15  60%
pen up 500 650
pen leave
wait 600
expect status.text is Nov 14, 22:15  59%
# The shortcut, on any screen: a keyboard's S (a controller's South).
key 31 0
wait 300
expect settings.status_on is 0
expect status.shown is 0
key 304 0 1
wait 300
expect settings.status_on is 1
expect status.shown is 1
# The sleep screen never has it, and the panel is whole again at the wake.
sleep
wait 400
expect asleep is 1
expect status.shown is 0
wake
wait 600
expect asleep is 0
expect status.shown is 1
expect status.text is Nov 14, 22:15  59%
# Sideways: the same corner of the turned screen.
rotate 1
wait 500
expect rotation is 1
expect status.shown is 1
expect status.rect is 6 6 518 88
tap 70 70
wait 300
expect settings.status_on is 0
expect status.shown is 0
tap 70 70
wait 300
expect settings.status_on is 1
expect status.shown is 1
shot 05-sideways
quit
