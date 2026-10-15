# Toolbar clear (no confirmation), Display sheet: brightness/warmth sliders, invert colors
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
pen curve 300 520 40 70 0.7 5
pen line 300 700 1300 720 60 0.3 0.9
wait 300
shot 01-before-clear
pentap 1768 810
wait 300
shot 02-cleared-by-toolbar
pentap 1768 594
wait 300
pentap 1768 1350
wait 300
shot 03-display-sheet
drag touch 1000 598 1500 598 5
wait 300
tap 400 878
wait 500
shot 04-inverted
tap 1464 1346
wait 300
shot 05-sheet-closed-inverted
quit
