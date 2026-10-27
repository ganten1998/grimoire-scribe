# Toolbar clear (no confirmation), Display sheet: brightness/warmth sliders, invert colors
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen curve 300 520 40 70 0.7 5
pen line 300 700 1300 720 60 0.3 0.9
wait 300
shot 01-before-clear
expect editor.strokes is 2
pentap 1768 810
wait 300
expect toast is Page cleared
shot 02-cleared-by-toolbar
expect editor.strokes is 0
pentap 1768 594
wait 300
expect editor.strokes is 2
pentap 1768 1350
wait 300
shot 03-display-sheet
expect editor.modal is display
expect settings.invert is 0
expect light.intensity is 12
# A slider being dragged redraws its own row alone (the whole card was a 1.8 Mpx copy at every step),
# at most every 80 ms; the release draws the whole card.
touch down 1000 598
wait 100
touch move 1100 598
expect display.last_fast_rect is 220 542 1420 140
touch up 1100 598
wait 300
drag touch 1000 598 1500 598 5
wait 300
expect light.intensity not 12
tap 400 878
wait 500
shot 04-inverted
expect settings.invert is 1
expect editor.modal is display
tap 1464 1624
wait 300
shot 05-sheet-closed-inverted
expect editor.modal is none
expect settings.invert is 1
quit
