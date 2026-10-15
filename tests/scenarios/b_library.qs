# Two notebooks with ink, a folder typed on the keyboard, move into it, settings, pen feel, calibration
wait 200
tap 965 95
wait 100
tap 790 460
tap 1564 2254
wait 300
pen curve 300 700 45 80 0.6 4
pen line 300 900 1300 950 80 0.2 0.9
pentap 1768 1566
wait 400
tap 965 95
wait 100
tap 1066 893
tap 1564 2254
wait 300
pentap 1768 358
pentap 1188 561
pen curve 300 420 30 60 0.7 3
pen line 300 520 1300 520 60 0.7 0.7
pentap 1768 1566
wait 400
shot 01-library-thumbs
# New folder "School"
tap 1155 95
wait 100
tap 175 2205
tap 398 2045
tap 747 2205
tap 1106 2045
tap 1549 1885
tap 1549 1885
tap 1637 2045
wait 100
shot 02-keyboard-school
tap 1643 2365
wait 300
shot 03-folder-created
# Move the newest notebook (card 1) into School
tap 1151 1052
wait 200
shot 04-more-menu
tap 930 988
wait 200
shot 05-move-menu
tap 930 935
wait 300
shot 06-moved
tap 326 608
wait 300
shot 07-inside-folder
tap 125 95
wait 300
# Settings sheet
tap 1535 95
wait 200
shot 08-settings
tap 930 446
wait 400
shot 09-penfeel-pencil
tap 1152 289
wait 200
drag touch 1000 1789 1500 1789 6
wait 200
pen line 700 600 1500 700 80 0.2 1.0
pen curve 700 820 30 60 0.8 4
wait 200
shot 10-penfeel-ballpoint
tap 125 95
wait 300
# Calibration
tap 1535 95
wait 200
tap 930 610
wait 300
shot 11-calib-first
pentap 223 298
pentap 1637 298
pentap 1637 2182
pentap 223 2182
pentap 930 1240
wait 200
shot 12-calib-touch
tap 223 298
tap 1637 298
tap 1637 2182
tap 223 2182
tap 930 1240
wait 200
pen line 200 1300 1600 1350 40 0.6 0.6
wait 100
shot 13-calib-verify
pentap 930 1585
wait 400
shot 14-after-calib
quit
