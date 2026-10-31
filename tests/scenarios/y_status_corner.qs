# settings: status_size=0
# settings: status_mode=1
# settings: status_on=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/status-keys .
# env: TZ=UTC
# The clock and battery corner (status.rs, the Rust build only): a small time, a pixel art battery and its
# percent in a corner of the NOTEBOOK PAGE and nowhere else. A finger tap in the corner, a 14 mm square, with no
# control there turns it on and off at once; the status-toggle shortcut does the same; the Display sheet sets its
# size. It is not on the Library, Settings, a sheet, the Pages view, the Links bar or the keyboard. It is an
# overlay: a small GL16 of its own rect at each minute and when the battery or its charging changes, held back
# while the pen is near or a stroke is drawn, and never in the sleep screen. The script sets the clock
# (2023-11-14 22:13:20 UTC) and the battery.
# (The first 4 s are the startup's two unconditional repaints, which this checks the refreshes of.)
# rust-only
wait 3800
clock 1700000000
battery 87
wait 100
expect settings.status_on is 0
expect settings.status_size is 0
expect settings.status_corner is 0
expect status.shown is 0
expect status.text is
expect status.rect is
expect status.front is 0
expect status.zone is 0 0 165 165
# The Library has no corner: a tap there is nothing to it (it is the bat's, which opens What's new), and
# the Display sheet's size shows nothing on it.
tap 70 70
wait 300
expect settings.status_on is 0
expect status.shown is 0
expect library.sheet is about
tap library about.close
wait 100
expect library.sheet is none
# Its Settings, where the size is set, and the controller's button turn it on: it is a setting, and the Library
# does not show it.
key 31 0
wait 300
expect settings.status_on is 1
expect status.shown is 0
expect toast is Clock and battery on. They show on a notebook page.
key 31 0
wait 300
expect settings.status_on is 0
expect toast is Clock and battery off.
# The editor: a notebook, the page in front.
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect status.front is 1
expect editor.modal is none
expect status.shown is 0
# A tap in the corner turns it on, at once: one small refresh of its box and nothing else.
expect display.presents not -1
tap 70 70
wait 300
expect settings.status_on is 1
expect status.shown is 1
expect status.text is Nov 14, 22:13  87%
expect status.charging is 0
expect status.rect is 6 6 331 48
expect status.icon is 218 16 54 27
expect status.fill is 10
expect display.presents is ui 6 6 331 48
shot 01-page-on
# A tap on the indicator itself turns it off (its box is part of the corner), and the refresh takes it
# down where it was.
tap 250 30
wait 300
expect settings.status_on is 0
expect status.shown is 0
expect display.presents is ui 6 6 331 48
tap 70 70
wait 300
expect status.shown is 1
expect display.presents is ui 6 6 331 48
# Not a tap: a finger that stays down, one that swipes out of the corner, one that comes down outside it,
# the pen.
touch down 70 70
wait 900
touch up 70 70
wait 200
expect settings.status_on is 1
drag touch 70 70 500 70 4
wait 200
expect settings.status_on is 1
tap 200 200
wait 200
expect settings.status_on is 1
pentap 70 70
pen leave
wait 200
expect settings.status_on is 1
expect display.presents not -1
# Display previews the clock live while its controls change; a tap
# in the corner is the sheet's, not the clock's. Display: the size, in a cell beside Done (the editor's sheet
# has the Pin this page row, so it is a row taller than the Library's).
tap panel display
wait 400
expect editor.modal is display
expect status.front is 0
expect status.shown is 1
tap 70 70
wait 300
expect editor.modal is none
expect settings.status_on is 1
expect status.shown is 1
tap panel display
wait 400
expect status.shown is 1
expect display.presents not -1
tap 472 1860
wait 300
expect settings.status_size is 1
expect status.shown is 1
tap 472 1860
wait 300
expect settings.status_size is 2
tap 472 1860
wait 300
expect settings.status_size is 0
tap 472 1860
wait 300
tap 472 1860
wait 300
expect settings.status_size is 2
shot 02-display-sheet-large
tap 1460 1860
wait 400
expect editor.modal is none
expect status.shown is 1
expect status.rect is 6 6 624 88
expect status.icon is 401 23 108 54
shot 03-page-large
# Charging: the bolt is inside the battery, so the box does not widen; a battery that changes is one refresh, a
# minute passing another, and neither a second.
expect display.presents not -1
battery 86 charging
wait 300
expect status.text is Nov 14, 22:13  86%
expect status.charging is 1
expect status.fill is 10
expect display.presents is ui 6 6 624 88
clock 1700000030
wait 300
expect display.presents is
clock 1700000060
wait 300
expect status.text is Nov 14, 22:14  86%
expect display.presents is ui 6 6 624 88
battery none
wait 300
expect status.text is Nov 14, 22:14
expect status.charging is 0
expect status.icon is
expect status.fill is
expect display.presents is ui 6 6 624 88
battery 100
wait 300
expect status.text is Nov 14, 22:14  100%
expect status.fill is 12
shot 04-large-100
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
expect display.presents is ui 6 6 650 88
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
expect status.rect is 6 6 624 88
tap 70 70
wait 300
expect settings.status_on is 0
expect status.shown is 0
tap 70 70
wait 300
expect settings.status_on is 1
expect status.shown is 1
shot 05-sideways
# Back in the Library it is gone from the screen and stays a setting.
tap panel library
wait 600
expect screen is library
expect status.shown is 0
expect settings.status_on is 1
quit
