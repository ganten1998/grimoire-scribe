# Live ink and the colour panel's wait (display_fbink.c): on a Scribe Colorsoft every fast refresh
# but live ink waits until the driver has read the frame, so a pen stroke must be marked live ink
# from its start to its end wherever it draws: ink and the eraser in the editor, the pen feel pad,
# the calibration's check box. A pressed button and a slider are not live ink. Only the editor's
# ink asks for the MTK pen mode (on the Scribe 2 that stays as it was). The simulated panel
# answers display.ink, display.pen_mode and display.last_fast.
# rust-only
wait 300
expect display.ink is 0
expect display.pen_mode is 0
# A new notebook (medium rule)
tap 965 95
wait 200
expect library.sheet is new
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.strokes is 0
# Ink: live from the pen's touch to its lifting, in the pen mode
pen down 300 700 0.6
pen move 400 720 0.6
pen move 500 740 0.6
expect display.ink is 1
expect display.pen_mode is 1
expect display.last_fast is ink
pen move 600 760 0.6
pen up 600 760
wait 700
expect display.ink is 0
expect display.pen_mode is 0
expect editor.strokes is 1
# The eraser end: live too (its frames never wait), without the pen mode
eraser down 300 760 0.6
eraser move 450 730 0.6
eraser move 650 760 0.6
expect display.ink is 1
expect display.pen_mode is 0
expect display.last_fast is ink
expect editor.strokes is 0
eraser up 650 760
wait 700
expect display.ink is 0
expect editor.strokes is 0
# A pressed panel button (Pages) is not live ink: a colour panel waits after it
touch down 1768 1026
expect display.last_fast is wait
touch up 1768 1026
wait 300
expect editor.modal is pages
tap 125 95
wait 300
expect editor.modal is none
# Pen feel (Library, Settings, Pen feel): the test pad's stroke is live ink, without the pen mode
pentap panel library
wait 500
expect screen is library
tap 1535 95
wait 200
expect library.sheet is settings
tap 930 446
wait 400
expect screen is penset
pen down 700 600 0.6
pen move 800 640 0.7
pen move 900 680 0.8
expect display.ink is 1
expect display.pen_mode is 0
expect display.last_fast is ink
pen move 1000 700 0.8
pen up 1000 700
wait 300
expect display.ink is 0
# A slider dragged is not live ink
touch down 1000 1789
touch move 1150 1789
expect display.last_fast is wait
expect display.ink is 0
touch up 1150 1789
wait 200
tap 125 95
wait 300
expect screen is library
# Calibration: the check box's stroke is live ink, without the pen mode (plain DU, as before)
tap 1535 95
wait 200
tap 930 610
wait 300
expect screen is calib
pentap 223 298
pentap 1637 298
pentap 1637 2182
pentap 223 2182
pentap 930 1240
wait 200
tap 223 298
tap 1637 298
tap 1637 2182
tap 223 2182
tap 930 1240
wait 200
pen down 300 1300 0.6
pen move 600 1320 0.6
pen move 900 1340 0.6
expect display.ink is 1
expect display.pen_mode is 0
expect display.last_fast is ink
pen up 900 1340
wait 100
expect display.ink is 0
expect screen is calib
# Cancel (keep the old calibration)
pentap 930 1985
wait 400
expect screen is library
expect settings.pen.valid is 0
expect display.ink is 0
quit
