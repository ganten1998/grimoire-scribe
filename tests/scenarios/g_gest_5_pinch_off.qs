# settings: status_mode=1
# rust-only
# Pinch zoom off: the swipes work (Pages opens and closes), a spread zooms nothing, and sideways with
# the page filling the width a vertical two-finger drag still pans.
# settings: pinch_zoom=0
# settings: landscape_fit=0
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
swipe 2 930 900 0 400
expect editor.modal is map
swipe 2 930 1400 0 -400
expect editor.modal is none
finger 1 down 800 1200
finger 2 down 1100 1200
frame 20
finger 1 move 750 1200
finger 2 move 1150 1200
frame 160
finger 1 move 650 1200
finger 2 move 1250 1200
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
expect editor.zoom is 1.00
expect editor.zoomed is 0
expect editor.modal is none
rotate 1
wait 400
expect editor.zoomed is 0
swipe 2 1240 1200 0 -400
expect editor.modal is none
expect editor.zoomed is 1
expect settings.pinch_zoom is 0
