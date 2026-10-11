# Toolbar clear (no confirmation), Display sheet: brightness/warmth sliders, invert colors
wait 200
tap 965 95
wait 100
tap 765 656
tap 1564 1934
wait 300
pen curve 300 520 40 70 0.7 5
pen line 300 700 1300 720 60 0.3 0.9
wait 300
shot 01-before-clear
pentap 1768 1134
wait 300
shot 02-cleared-by-toolbar
pentap 1768 918
wait 300
pentap 1768 1674
wait 300
shot 03-display-sheet
drag touch 1000 851 1500 851 5
wait 300
tap 400 1151
wait 500
shot 04-inverted
tap 1464 1346
wait 300
shot 05-sheet-closed-inverted
quit
