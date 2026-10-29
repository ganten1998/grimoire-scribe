# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# root: s_shape_1_record
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pen line 300 600 1300 610 10 0.6 0.6
pen leave
wait 1100
expect editor.strokes is 1
# Proximity blocks a shape even where its first point would otherwise arm.
pen hover 930 1100
shape Z 930 1150 45 600
expect editor.strokes is 1
expect fshape.fired is 0
pen leave
wait 1100
# A start on the panel never becomes a shape on the page.
fpath 1768 900 1400 900 1400 1200 1000 1200 600
expect fshape.fired is 0
expect fshape.trail is
# A second finger after 200 ms cancels the existing trail.
wait 1100
finger 1 down 600 900
frame 20
wait 20
finger 1 move 650 900
frame 40
wait 40
finger 1 move 700 900
frame 40
wait 40
finger 1 move 750 900
frame 40
wait 40
finger 1 move 800 900
frame 40
wait 40
finger 1 move 850 900
frame 20
wait 20
expect fshape.state is drawing
expect fshape.trail not
finger 2 down 1100 900
frame 20
wait 20
# An uncommitted pinch restores the cancelled trail at PinchEnd.
finger 1 up 850 900
finger 2 up 1100 900
frame
wait 100
expect fshape.trail is
expect fshape.fired is 0
# Late start: a still finger for 450 ms cannot then become a shape.
finger 1 down 600 900
frame 450
finger 1 move 1100 900
frame 30
finger 1 up 1100 900
frame
expect fshape.fired is 0
# Stay in motion for over three seconds; the trail thins while the contact is still down.
wait 1100
expect display.presents not clear
finger 1 down 600 900
frame 20
wait 20
finger 1 move 850 900
frame 200
wait 200
finger 1 move 850 1150
frame 200
wait 200
finger 1 move 600 1150
frame 200
wait 200
finger 1 move 600 900
frame 200
wait 200
finger 1 move 850 900
frame 200
wait 200
expect display.presents has fast
finger 1 move 850 1150
frame 200
wait 200
finger 1 move 600 1150
frame 200
wait 200
finger 1 move 600 900
frame 200
wait 200
finger 1 move 850 900
frame 200
wait 200
finger 1 move 850 1150
frame 200
wait 200
expect display.presents has fast
finger 1 move 600 1150
frame 200
wait 200
finger 1 move 600 900
frame 200
wait 200
finger 1 move 850 900
frame 200
wait 200
finger 1 move 850 1150
frame 200
wait 200
finger 1 move 600 1150
frame 200
wait 200
finger 1 move 600 900
frame 200
wait 200
finger 1 move 850 900
frame 200
wait 200
expect fshape.state is dead
expect fshape.trail not
expect display.presents has fast
finger 1 up 850 900
frame
wait 1100
expect fshape.trail is
expect fshape.fired is 0
# A 600 ms pause also cancels a moving shape.
finger 1 down 600 900
frame 20
finger 1 move 850 900
frame 30
finger 1 move 850 900
frame 600
finger 1 move 1200 900
frame 30
finger 1 up 1200 900
frame
expect fshape.fired is 0
expect editor.strokes is 1
# A shape starts inside the clock corner and still acts; a tap only toggles the clock.
shape Z 300 300 45 600
expect editor.strokes is 0
expect fshape.fired is 1
expect settings.status_on is 0
tap 60 60
expect settings.status_on is 1
tap 60 60
expect settings.status_on is 0
# A normal link tap/hold keeps its existing behavior while shapes are armed.
pentap panel next
pentap panel previous
key 38 0
pen line 300 1000 900 1200 20 0.6 0.6
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
tap 703 340
expect editor.links is 1 1
pen leave
wait 1100
touch down 600 1100
wait 650
expect editor.link_card is 1
touch up 600 1100
tap 1300 1900
wait 100
expect editor.link_card is 0
tap 600 1100
expect editor.page is 2
