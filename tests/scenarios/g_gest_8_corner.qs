# rust-only
# The clock corner's tap (a finger, 14 mm square, top left) is never claimed: not by a link drawn there,
# not by a gesture that just fired, and a two-finger swipe or tap that starts there is a gesture and
# leaves the clock alone.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/link-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect settings.status_on is 0
tap 60 60
expect settings.status_on is 1
expect editor.page is 1
expect editor.modal is none
tap 60 60
expect settings.status_on is 0
# right after a gesture that fired (a turn past the end: the toast), the corner's tap is still the clock's
wait 1100
swipe 2 1300 1200 -400 0
expect toast is This is the last page.
tap 60 60
expect settings.status_on is 1
tap 60 60
expect settings.status_on is 0
# with the Links bar up (an empty one: no links on this page) the clock is not shown, and its corner is not the
# clock's: a tap there is any tap outside the bar, which stays on the page and closes the bar
wait 200
swipe 2 930 1400 0 -400
expect editor.bar is open
tap 60 60
expect settings.status_on is 0
expect editor.bar is 0
expect editor.modal is none
# a two-finger swipe whose first finger lands in the corner: a gesture, not the clock
wait 200
finger 1 down 60 60
frame 30
finger 2 down 360 60
frame 20
finger 1 move 60 100
finger 2 move 360 100
frame 20
finger 1 move 60 140
finger 2 move 360 140
frame 20
finger 1 move 60 180
finger 2 move 360 180
frame 20
finger 1 move 60 220
finger 2 move 360 220
frame 20
finger 1 move 60 260
finger 2 move 360 260
frame 20
finger 1 move 60 300
finger 2 move 360 300
frame 20
finger 1 move 60 340
finger 2 move 360 340
frame 20
finger 1 move 60 380
finger 2 move 360 380
frame 20
finger 1 move 60 420
finger 2 move 360 420
frame 20
finger 1 move 60 460
finger 2 move 360 460
frame 20
finger 1 up 60 460
frame 5
finger 2 up 360 460
frame
expect touch.gesture is 2.down
expect editor.modal is pages
expect settings.status_on is 0
swipe 2 930 1400 0 -400
expect editor.modal is none
# a two-finger tap whose midpoint is in the corner is a two-finger tap
tapn 2 100 100
expect touch.gesture is 2.tap
expect settings.status_on is 0
expect editor.modal is none
# a link drawn over the corner: its tap there is the clock's, and the link is not followed; the rest of
# its box still follows it
pentap panel next
wait 100
pentap panel previous
wait 100
expect editor.pages is 2
key 38 0
pen line 40 40 300 300 30 0.6 0.6
expect editor.modal is pages
expect editor.picking is new
tap 703 340
expect editor.modal is none
expect editor.links is 1 1
expect editor.link.1 is p2
tap 60 60
expect settings.status_on is 1
expect editor.page is 1
tap 60 60
expect settings.status_on is 0
expect editor.page is 1
tap 250 250
expect editor.page is 2
