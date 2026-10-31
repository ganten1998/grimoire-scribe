# settings: status_size=0
# settings: status_mode=1
# settings: status_on=0
# settings: shortcuts_enabled=1
# env: TZ=UTC
# rust-only
# Taps in the clock and battery corner are reliable: one finger tap always toggles it, at once, with no lockout.
# The cases that used to fail (the owner on a Kindle: "inconsistent and janky ... an odd delay before I can try
# again"): a fingertip that rolls on the glass past the buttons' 2.5 mm, a lift a hair outside the square, a pen that
# left without a word (its "near" held the refresh back, so the setting changed and nothing showed), and a tap
# right after a tap. The zone is a 14 mm square and the box's place with 4 mm around it, shown or not (the owner,
# on beta.5's first build: half the taps to bring it back went where the box had been, outside the square); the slop
# is 5 mm.
wait 3800
clock 1700000000
battery 87
wait 100
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect settings.status_on is 0
expect status.zone is 0 0 165 165
# The edges of the zone: the last pixel inside takes it, the first one outside does not. At this size the box is
# 6 6 331 48; with 4 mm (47 px) around it, its place ends at x 384 and y 101 (a few pixels kept clear of those
# edges: the runner's touch panel rounds a tap's place by a pixel).
tap 164 164
wait 200
expect settings.status_on is 1
expect status.shown is 1
expect status.rect is 6 6 331 48
tap 600 600
wait 100
tap 165 104
wait 200
expect settings.status_on is 1
tap 100 165
wait 200
expect settings.status_on is 1
tap 388 30
wait 200
expect settings.status_on is 1
tap 20 20
wait 200
expect settings.status_on is 0
expect status.shown is 0
# Off, a tap where the box shows, at its far end and near the end of the 4 mm beyond it, brings it back.
tap 330 40
wait 200
expect settings.status_on is 1
tap 20 20
wait 200
expect settings.status_on is 0
tap 380 97
wait 200
expect settings.status_on is 1
tap 20 20
wait 200
expect settings.status_on is 0
expect status.shown is 0
# A fingertip rolls: 3.8 mm and 4.7 mm away from where it landed are taps (the buttons' 2.5 mm slop is not the
# corner's), 5.9 mm is a drag.
touch down 100 100
touch move 130 120
touch up 140 110
wait 200
expect settings.status_on is 1
touch down 100 100
touch move 100 130
touch up 100 155
wait 200
expect settings.status_on is 0
touch down 100 100
touch move 100 140
touch up 100 170
wait 200
expect settings.status_on is 0
# A lift just outside the square is a tap too, where the finger landed inside it.
touch down 150 150
touch up 175 160
wait 200
expect settings.status_on is 1
touch down 150 150
touch up 215 150
wait 200
expect settings.status_on is 1
# A press that is held is not a tap (a hold is the page's), and a swipe out is not one.
touch down 70 70
wait 900
touch up 70 70
wait 200
expect settings.status_on is 1
# No lockout: eight taps one after the other, 40 ms apart, each its own toggle, each shown at once.
tap 70 70
wait 40
expect settings.status_on is 0
tap 70 70
wait 40
expect settings.status_on is 1
tap 70 70
wait 40
expect settings.status_on is 0
tap 70 70
wait 40
expect settings.status_on is 1
tap 70 70
wait 40
expect settings.status_on is 0
tap 70 70
wait 40
expect settings.status_on is 1
tap 70 70
wait 40
expect settings.status_on is 0
tap 70 70
wait 40
expect settings.status_on is 1
wait 300
expect status.shown is 1
# At once: a refresh of the box goes out within the tick that follows the lift, so a tap that comes 120 ms later
# finds it shown, and takes it down, and the box is gone 120 ms after that.
expect display.presents not -1
tap 70 70
wait 120
expect status.shown is 0
expect display.presents is ui 6 6 331 48
tap 70 70
wait 120
expect status.shown is 1
expect display.presents is ui 6 6 331 48
# The setting is kept after the refresh, so a restart finds it as the last tap left it.
expect settings.status_on is 1
# Through the touch recognizer as the Kindle's panel goes through it (contact frames): one finger, one tap.
finger 1 down 70 70
frame 40
finger 1 up 70 70
frame 13
wait 200
expect settings.status_on is 0
expect touch.contacts is 0
finger 1 down 70 70
frame 90
finger 1 move 84 80
frame 13
finger 1 up 90 82
frame 13
wait 200
expect settings.status_on is 1
# A pen that left without a word (the runner never sends the leaving of a pentap's pen): the finger that follows is
# not its hand, and the refresh is sent, not held for a "near" that is over.
pentap 900 900
wait 1700
tap 70 70
wait 300
expect settings.status_on is 0
expect status.shown is 0
tap 70 70
wait 300
expect settings.status_on is 1
expect status.shown is 1
pen leave
# A control under the finger keeps the tap: the panel dragged into the corner is the panel's, and the corner
# is the clock's again once it is dragged out.
drag touch 1737 260 33 62 5
wait 400
expect editor.panel_at.30.62 is grip
tap 30 62
wait 300
expect settings.status_on is 1
expect editor.panel_at.30.62 is grip
drag touch 33 62 1737 260 5
wait 400
tap 70 70
wait 300
expect settings.status_on is 0
quit
