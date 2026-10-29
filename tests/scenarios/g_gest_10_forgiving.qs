# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect touch.tune is new
finger 1 down 650 1500
finger 2 down 1250 1500
frame 20
finger 1 move 650 1400
finger 2 move 1250 1400
frame 20
finger 1 move 662 1350
finger 2 move 1238 1350
frame 13
finger 1 move 674 1300
finger 2 move 1226 1300
frame 13
finger 1 move 686 1250
finger 2 move 1214 1250
frame 13
finger 1 move 698 1200
finger 2 move 1202 1200
frame 13
finger 1 move 710 1150
finger 2 move 1190 1150
frame 13
finger 1 move 722 1100
finger 2 move 1178 1100
frame 13
finger 1 move 734 1050
finger 2 move 1166 1050
frame 13
finger 1 move 746 1000
finger 2 move 1154 1000
frame 13
finger 1 move 758 950
finger 2 move 1142 950
frame 13
finger 1 move 770 900
finger 2 move 1130 900
frame 13
# The 600 px separation converges by 240 px (40%).
finger 1 up 770 900
finger 2 up 1130 900
frame
expect touch.gesture is 2.up
expect editor.bar is open
swipe 2 930 1000 0 400
expect editor.bar is 0
finger 1 down 700 1400
finger 2 down 1000 1400
frame 40
finger 1 move 700 1310
finger 2 move 1000 1310
frame 30
finger 1 up 700 1310
frame 0
finger 2 move 1000 1015
frame 40
finger 2 up 1000 1015
frame
expect touch.gesture is 2.up
expect editor.bar is open
swipe 2 930 1000 0 400
expect editor.bar is 0
# Midpoint drift and swipe-to-spread promotion both zoom.
finger 1 down 700 1310
finger 2 down 1000 1310
frame 20
finger 1 move 688 1110
finger 2 move 1028 1110
frame 13
finger 1 move 676 1120
finger 2 move 1056 1120
frame 13
finger 1 move 664 1130
finger 2 move 1084 1130
frame 13
finger 1 move 652 1140
finger 2 move 1112 1140
frame 13
finger 1 move 640 1150
finger 2 move 1140 1150
frame 13
finger 1 move 628 1160
finger 2 move 1168 1160
frame 13
finger 1 move 616 1170
finger 2 move 1196 1170
frame 13
finger 1 move 604 1180
finger 2 move 1224 1180
frame 13
finger 1 move 592 1190
finger 2 move 1252 1190
frame 13
finger 1 move 580 1200
finger 2 move 1280 1200
frame 13
finger 1 up 580 1200
finger 2 up 1280 1200
frame
expect editor.zoom not 1.00
key 33 0
expect editor.zoom is 1.00
finger 1 down 800 1200
finger 2 down 1100 1200
frame 20
finger 1 move 890 1200
finger 2 move 1190 1200
frame 20
expect touch.class is swipe
# Motion reverses along the candidate's own axis, becoming an actual spread.
finger 1 move 750 1200
finger 2 move 1300 1200
frame 20
expect touch.class is zoom
finger 1 move 700 1200
finger 2 move 1350 1200
frame 20
finger 1 move 650 1200
finger 2 move 1400 1200
frame 20
finger 1 up 650 1200
finger 2 up 1400 1200
frame
expect editor.zoom not 1.00
# A brief lost contact preserves the pinch without a UI settle.
finger 1 down 700 1000
finger 2 down 1000 1000
frame 20
finger 1 move 650 1000
finger 2 move 1050 1000
frame 40
finger 1 move 600 1000
finger 2 move 1100 1000
frame 40
expect display.presents not clear
finger 2 up 1100 1000
frame 25
expect display.presents is
finger 2 down 1100 1000
frame 20
expect touch.class is zoom
expect display.presents is
finger 1 move 550 1000
finger 2 move 1150 1000
frame 40
finger 1 up 550 1000
finger 2 up 1150 1000
frame
key 33 0
expect editor.zoom is 1.00
# Let the prior synthetic contact clock catch up before measuring a real 400 ms pen gap.
wait 2000
pen line 250 400 400 410 8 0.6 0.6
pen leave
wait 400
# The pinch begins over 60 mm from the last ink point.
finger 1 down 1050 1500
finger 2 down 1350 1500
frame 20
finger 1 move 950 1500
finger 2 move 1450 1500
frame 20
finger 1 move 900 1500
finger 2 move 1500 1500
frame 20
finger 1 move 850 1500
finger 2 move 1550 1500
frame 20
expect touch.class is zoom
finger 1 up 850 1500
finger 2 up 1550 1500
frame
expect touch.reject is
expect editor.zoom not 1.00
