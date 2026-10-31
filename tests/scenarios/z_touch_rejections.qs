# rust-only
# Touch rejection through the recognizer (finger and frame lines, as the Kindle's panel goes
# through touchgest): the pen's hand is held only over the notebook's ink, never on the Library
# or a sheet, and a finger far from a hovering pen counts even on the page.
wait 300
expect screen is library
# The Library: a finger tap while the pen hovers far away opens New folder's keyboard.
pen hover 900 2300
finger 1 down 1155 95
frame 40
finger 1 up 1155 95
frame
wait 50
expect library.sheet is keyboard
tap 217 2365
wait 50
# A pen hover after it left, 600 ms on: still a finger off the ink.
pen leave
wait 600
finger 1 down 1155 95
frame 40
finger 1 up 1155 95
frame
wait 50
expect library.sheet is keyboard
tap 217 2365
wait 50
expect library.sheet is none
# The keyboard: two fingers overlapping as they type (the second lands before the first lifts) type
# both keys, in order.
tap 1155 95
wait 50
expect library.sheet is keyboard
finger 1 down 221 2045
frame 60
finger 2 down 398 2045
frame 40
finger 1 up 221 2045
frame 40
finger 2 up 398 2045
frame
wait 50
expect library.text is as
tap 217 2365
wait 50
# Pen feel: a slider a second finger takes over (a palm, landing late) keeps the value it reached.
tap 1535 95
tap 930 446
expect screen is penset
finger 1 down 830 1172
frame 100
finger 1 move 880 1172
frame 500
expect penset.param.size_min_mm is 0.9000
finger 2 down 400 1900
frame 40
finger 2 up 400 1900
finger 1 up 880 1172
frame
wait 100
expect penset.param.size_min_mm is 0.9000
expect penset.slider_pending is 0
tap 125 95
wait 300
expect screen is library
# A notebook: on its panel a finger is a finger with the pen hovering beside it (20 mm away).
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.panel_at.1768.1674 is display
pen hover 1530 1674
finger 1 down 1768 1674
frame 40
finger 1 up 1768 1674
frame
wait 300
expect editor.modal is display
# The Display sheet: a brightness drag the pen's arrival cancels keeps the value it reached (a
# cancel's off-screen place once set the light to 0, and saved it).
expect light.intensity is 12
pen leave
finger 1 down 1000 552
frame 100
finger 1 move 1100 552
frame 100
finger 1 move 1150 552
frame 100
pen hover 300 1800
wait 300
expect light.intensity is 10
finger 1 up 1150 552
frame
tap 1464 1858
wait 300
expect editor.modal is none
# On the page's ink a finger beside the hovering pen is its writing hand, held; one far from it
# (a fresh hover 60 mm away) is a finger.
pen hover 900 1200
finger 1 down 950 1250
frame 40
expect touch.suppressed is 1
finger 1 up 950 1250
frame
pen hover 300 300
finger 1 down 1200 2000
frame 20
expect touch.class is one
finger 1 up 1200 2000
frame
pen leave
