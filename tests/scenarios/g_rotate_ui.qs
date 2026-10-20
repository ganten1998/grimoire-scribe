# Library and sheets when the Kindle is turned sideways
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
pen curve 300 520 40 70 0.7 5
pentap 1768 1566
wait 500
expect screen is library
expect library.items is 1
rotate 1
wait 500
shot 01-library-landscape
expect rotation is 1
expect size is 2480x1860
expect screen is library
tap 2155 95
wait 300
shot 02-settings-landscape
expect library.sheet is settings
rotate 3
wait 500
shot 03-settings-landscape-other-way
expect rotation is 3
expect size is 2480x1860
expect screen is library
rotate 0
wait 500
shot 04-portrait
expect rotation is 0
expect size is 1860x2480
quit
