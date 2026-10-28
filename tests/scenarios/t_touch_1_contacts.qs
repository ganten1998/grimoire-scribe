# settings: status_mode=1
# rust-only
# Contact-level touch through the recognizer the Kindle's panel goes through: a spread zooms like the
# legacy pinch, a palm (tool 2) keeps every contact quiet until all lift, and a pen coming near ends a
# finger without a tap or a link follow.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
# a spread
finger 1 down 800 1200
finger 2 down 1100 1200
frame 20
finger 1 move 750 1200
finger 2 move 1150 1200
frame 160
finger 1 move 700 1200
finger 2 move 1200 1200
frame 160
finger 1 move 650 1200
finger 2 move 1250 1200
frame 160
finger 1 move 600 1200
finger 2 move 1300 1200
frame 160
finger 1 move 550 1200
finger 2 move 1350 1200
frame 160
finger 1 move 500 1200
finger 2 move 1400 1200
frame 160
finger 1 up 500 1200
frame 10
finger 2 up 1400 1200
frame
expect touch.contacts is 0
expect touch.class is none
expect editor.zoomed is 1
expect editor.zoom not 1.00
wait 400
# a palm: every contact waits to lift, a second one landing and lifting does not change that
finger 1 down 900 1200 0 2
frame
expect touch.suppressed is 1
finger 2 down 1200 1500
frame 20
expect touch.suppressed is 1
finger 2 up 1200 1500
frame 20
expect touch.suppressed is 1
finger 1 up 900 1200
frame
expect touch.suppressed is 0
expect touch.contacts is 0
expect editor.modal is none
# the pen coming near ends a finger without a tap or a follow
finger 1 down 900 1500
frame
pen hover 300 300
finger 1 up 900 1500
frame
expect touch.class is none
expect touch.contacts is 0
expect editor.modal is none
# a panel button pressed by one finger and taken over by a second is let go with a Fast refresh, as it
# was pressed (the first finger's cancel, then the pinch), not a Ui one
pen leave
wait 1100
expect editor.panel_at.1768.1026 is pages
expect display.presents not -1
finger 1 down 1768 1026
frame 40
expect display.presents is fast 1712 996 112 60
finger 2 down 1400 1026
frame 20
expect display.presents is fast 1712 996 112 60
finger 1 up 1768 1026
finger 2 up 1400 1026
frame
expect touch.contacts is 0
expect editor.modal is none
