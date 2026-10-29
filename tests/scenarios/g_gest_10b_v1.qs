# rust-only
# env: GRIMOIRE_TOUCH_TUNE=v1
# settings: mirror_pdf=0
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect touch.tune is v1
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
expect touch.reject is spread
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
expect touch.reject is slow
expect editor.modal is none
