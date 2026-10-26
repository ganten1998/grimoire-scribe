# Pen feel with white ink on (white=1, as the tool picker's ink row sets it) and its Eraser row.
# A drawing tool's tab has a black swatch on its test pad, and the pen draws white there with the
# tool's look (penset.pad_white: white went through the swatch); Clear lays it again. The eraser's
# tab has TEST and a block, and its row, Whole strokes | Precise, sets settings.eraser_mode: with Precise
# the pen's eraser end on the pad takes away only what it touches (of the swatch here, the pad's
# own lines kept), with Whole strokes it clears the pad (and draws nothing).
# settings: white=1
wait 300
tap 1535 95
wait 200
expect library.sheet is settings
tap 930 446
wait 400
expect screen is penset
expect settings.white is 1
expect settings.eraser_mode is 0
# The pencil's tab: the swatch, untouched, then white pencil across it
expect penset.pad_white is 0
expect penset.pad_ink is 1
shot 01-pad-swatch
pen line 700 640 1500 700 40 0.9 0.9
wait 300
expect penset.pad_white is 1
expect penset.pad_lighter is 0
shot 02-white-pencil-on-the-swatch
# Clear: the swatch whole again
tap 1708 498
wait 200
expect penset.pad_white is 0
# The charcoal's tab: its white has the charcoal's grain
tap 484 289
wait 200
expect penset.pad_white is 0
pen line 700 640 1500 760 40 1.0 1.0
wait 300
expect penset.pad_white is 1
shot 03-white-charcoal
# The eraser's tab has fresh handwritten ink; its row picks how it erases
tap 1723 289
wait 200
expect penset.pad_white is
shot 04-eraser-row-whole
tap 1559 1440
wait 200
expect settings.eraser_mode is 1
shot 05-eraser-row-precise
# Back on the pencil's tab, the precise eraser end through the swatch: it goes there, the pad's
# lines and the rest of the swatch stay
tap 130 289
wait 200
expect penset.pad_white is 0
eraser line 1000 600 1060 860 12
wait 300
expect penset.pad_white is 1
expect penset.pad_lighter is 0
shot 06-precise-through-the-swatch
# Whole strokes again: the eraser end clears the pad, and draws nothing (the swatch is whole)
tap 1723 289
wait 200
tap 1039 1440
wait 200
expect settings.eraser_mode is 0
tap 130 289
wait 200
pen line 700 640 1500 700 40 0.9 0.9
wait 300
expect penset.pad_white is 1
eraser line 1000 600 1060 860 12
wait 300
expect penset.pad_white is 0
# Sideways: the eraser's row, Precise, then Whole strokes
rotate 1
wait 300
tap 2312 289
wait 200
shot 07-eraser-row-landscape
tap 2089 1440
wait 200
expect settings.eraser_mode is 1
tap 1389 1440
wait 200
expect settings.eraser_mode is 0
rotate 0
wait 300
tap 125 95
wait 300
expect screen is library
quit
