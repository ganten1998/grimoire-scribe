# Share turned sideways and back, then asleep and awake: the QR code beside the text in
# landscape, one server the whole visit (tests/share_check.sh downloads across the turns and
# checks it never restarts), stopped by Done and started again by the next visit.
wait 300
tap 1345 95
wait 700
expect screen is share
expect watching is 1
shot 01-share-portrait
wait 1000
rotate 1
wait 700
expect rotation is 1
expect size is 2480x1860
shot 02-share-landscape
wait 1000
rotate 0
wait 700
expect size is 1860x2480
shot 03-share-portrait-again
sleep
wait 500
expect asleep is 1
# Asleep, nothing accepts a connection, so the server is not watched (one waiting would make
# the main loop spin); the wake watches it again. With no page open, the sleep screen is the
# card without the code (nothing would answer a phone that scanned it).
expect watching is 0
shot 04-asleep-no-code
wake
wait 700
expect asleep is 0
expect screen is share
expect watching is 1
shot 05-share-after-wake
wait 2000
# Done
tap 1540 2105
wait 700
expect screen is library
expect watching is 0
shot 06-library
wait 1500
# Share again, opened in portrait and turned while it is open: this visit starts the server
# again and the turn keeps it (tests/share_check.sh counts one start per visit).
tap 1345 95
wait 700
expect screen is share
rotate 1
wait 700
shot 07-share-landscape-again
wait 1500
rotate 0
wait 700
tap 1540 2105
wait 700
expect screen is library
quit
