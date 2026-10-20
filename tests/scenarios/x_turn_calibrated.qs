# Taps and pen taps land where the script says after the Kindle turns, on a calibrated root too
# (b_library calibrated the pen and touch). As on the Kindle, the calibration maps the digitizer
# to the upright panel, so one made upright stays right sideways.
# root: b_library
wait 300
expect screen is library
expect settings.pen.valid is 1
expect settings.touch.valid is 1
rotate 1
wait 500
shot 01-library-landscape
expect rotation is 1
expect size is 2480x1860
# Settings and its Done with a finger, the School folder and back with the pen
tap 2155 95
wait 300
shot 02-settings-landscape
expect library.sheet is settings
tap 1908 1695
wait 300
expect library.sheet is none
pentap 267 530
wait 300
shot 03-folder-landscape
expect library.folder is School
pentap 125 95
wait 300
expect library.folder is
# Turned the other way: the folder with a finger, Settings with the pen
rotate 3
wait 500
shot 04-library-other-way
expect rotation is 3
expect size is 2480x1860
tap 267 530
wait 300
expect library.folder is School
tap 125 95
wait 300
expect library.folder is
pentap 2155 95
wait 300
shot 05-settings-other-way
expect library.sheet is settings
pentap 1908 1695
wait 300
expect library.sheet is none
rotate 0
wait 500
shot 06-portrait
expect rotation is 0
expect size is 1860x2480
quit
