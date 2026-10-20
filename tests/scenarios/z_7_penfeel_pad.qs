# The Pen feel test pad with the precise eraser (eraser_mode=1): the pen's eraser end, and the tip
# on the eraser's tab, erase only what they touch on the pad (its lines and Clear button stay), as
# in the editor, instead of clearing it. The pad's strokes are live ink. White ink (white=1) draws
# as ink on the pad, so it still shows each tool's feel.
# settings: eraser_mode=1
# settings: white=1
wait 300
tap 1535 95
wait 200
expect library.sheet is settings
tap 930 446
wait 400
expect screen is penset
pen line 700 520 1500 540 40 0.7 0.7
pen line 700 640 1500 660 40 0.7 0.7
pen line 700 760 1500 780 40 0.7 0.7
wait 300
# The eraser end across the three lines
eraser down 1000 460 0.6
eraser move 1030 600 0.6
eraser move 1060 760 0.6
expect display.ink is 1
eraser move 1080 900 0.6
eraser up 1080 900
wait 500
expect display.ink is 0
shot 01-pad-erased-precisely
# Over the Clear button and back: the button comes back as it was drawn
eraser line 1550 470 1750 560 20
wait 500
shot 02-pad-clear-button-kept
# The eraser's tab (the pad drawn bare again): the tip is the precise eraser there, and the pad's
# lines stay under it
tap 1723 289
wait 300
pen line 700 500 1500 800 40 0.6 0.6
wait 500
shot 03-eraser-tab-tip
expect screen is penset
tap 125 95
wait 300
expect screen is library
quit
