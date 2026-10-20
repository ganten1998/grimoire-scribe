# The Pen feel test pad with the precise eraser (eraser_mode=1): the pen's eraser end, and the tip
# on the eraser's tab, erase only what they touch on the pad (its lines and Clear button stay), as
# in the editor, instead of clearing it. The pad's strokes are live ink. White ink (white=1) draws
# as ink on the pad, so it still shows each tool's feel. penset.pad_ink: ink shows on the pad;
# penset.pad_ink_near.X.Y: ink shows near that point; penset.pad_lighter: a pixel of the pad is
# lighter than the pad drawn bare (one of its lines, or its Clear button, erased away).
# settings: eraser_mode=1
# settings: white=1
wait 300
tap 1535 95
wait 200
expect library.sheet is settings
tap 930 446
wait 400
expect screen is penset
expect penset.pad_ink is 0
expect penset.pad_lighter is 0
pen line 700 520 1500 540 40 0.7 0.7
pen line 700 640 1500 660 40 0.7 0.7
pen line 700 760 1500 780 40 0.7 0.7
wait 300
# White ink is ink here
expect penset.pad_ink is 1
expect penset.pad_ink_near.1014.528 is 1
expect penset.pad_ink_near.800.522 is 1
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
# Where it went the lines are gone, elsewhere they stay, and the pad's own lines are all there
expect penset.pad_ink_near.1014.528 is 0
expect penset.pad_ink_near.1043.648 is 0
expect penset.pad_ink_near.800.522 is 1
expect penset.pad_ink_near.1300.535 is 1
expect penset.pad_ink is 1
expect penset.pad_lighter is 0
# Over the Clear button and back: the button comes back as it was drawn
eraser line 1550 470 1750 560 20
wait 500
shot 02-pad-clear-button-kept
expect penset.pad_lighter is 0
expect penset.pad_ink is 1
# The eraser's tab (the pad drawn bare again): the tip is the precise eraser there, and the pad's
# lines stay under it
tap 1723 289
wait 300
expect penset.pad_ink is 0
pen line 700 500 1500 800 40 0.6 0.6
wait 500
shot 03-eraser-tab-tip
expect screen is penset
expect penset.pad_ink is 0
expect penset.pad_lighter is 0
tap 125 95
wait 300
expect screen is library
quit
