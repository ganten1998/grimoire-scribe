# Two notebooks with ink, a folder typed on the keyboard, move into it, settings, pen feel, calibration
wait 200
tap 965 95
wait 100
expect library.sheet is new
tap 790 460
expect library.paper is 2
tap 1564 2254
wait 300
expect screen is editor
expect editor.paper is 2
pen curve 300 700 45 80 0.6 4
pen line 300 900 1300 950 80 0.2 0.9
pentap 1768 1566
wait 400
expect screen is library
expect library.items is 1
tap 965 95
wait 100
expect library.sheet is new
tap 1066 893
expect library.paper is 3
tap 1564 2254
wait 300
expect screen is editor
expect editor.paper is 3
pentap 1768 358
expect editor.modal is tools
pentap 872 561
expect editor.tool is ballpoint
pen curve 300 420 30 60 0.7 3
pen line 300 520 1300 520 60 0.7 0.7
# The Library puts the latest saved first, by whole seconds: this notebook's save must come in a
# later second than the first one's, or a tie could put the older one first and move that.
wait 1100
pentap 1768 1566
wait 400
shot 01-library-thumbs
expect screen is library
expect library.items is 2
expect library.folder is
# New folder "School"
tap 1155 95
wait 100
expect library.sheet is keyboard
expect library.text is
tap 175 2205
tap 398 2045
tap 747 2205
tap 1106 2045
tap 1549 1885
tap 1549 1885
tap 1637 2045
wait 100
shot 02-keyboard-school
expect library.text is School
tap 1643 2365
wait 300
shot 03-folder-created
expect library.sheet is none
expect library.items is 3
# Move the newest notebook (card 1) into School
tap 1151 1052
wait 200
shot 04-more-menu
expect library.sheet is more
tap 930 988
wait 200
shot 05-move-menu
expect library.sheet is move
tap 930 935
wait 300
shot 06-moved
expect library.sheet is none
expect library.items is 2
tap 326 608
wait 300
shot 07-inside-folder
expect library.folder is School
expect library.items is 1
tap 125 95
wait 300
expect library.folder is
expect library.items is 2
# Settings sheet
tap 1535 95
wait 200
shot 08-settings
expect library.sheet is settings
tap 930 446
wait 400
shot 09-penfeel-ballpoint
expect screen is penset
tap 1152 289
wait 200
expect settings.fountain.pressure_gamma not 2.9000
drag touch 1000 1789 1500 1789 6
wait 200
# (1152, 289) is the Fountain pen's tab since the new pens came, and the drag set its pressure curve
expect settings.fountain.pressure_gamma is 2.9000
pen line 700 600 1500 700 80 0.2 1.0
pen curve 700 820 30 60 0.8 4
wait 200
shot 10-penfeel-fountain
expect screen is penset
tap 125 95
wait 300
expect screen is library
expect library.sheet is none
# Calibration
tap 1535 95
wait 200
expect library.sheet is settings
tap 930 610
wait 300
shot 11-calib-first
expect screen is calib
pentap 223 298
pentap 1637 298
pentap 1637 2182
pentap 223 2182
pentap 930 1240
wait 200
shot 12-calib-touch
expect screen is calib
tap 223 298
tap 1637 298
tap 1637 2182
tap 223 2182
tap 930 1240
wait 200
pen line 200 1300 1600 1350 40 0.6 0.6
wait 100
shot 13-calib-verify
expect screen is calib
expect settings.pen.valid is 0
pentap 930 1585
wait 400
shot 14-after-calib
expect screen is library
expect settings.pen.valid is 1
expect settings.touch.valid is 1
quit
