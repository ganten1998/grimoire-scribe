# settings: shortcuts_enabled=1
# settings: controller=1
# Pairing a Bluetooth controller through the helper's API: tests/bt_check.sh stands a helper in
# (tests/fake_bthelper.py) and runs this (a helper is needed, so it is not in tests/scenarios). The helper is
# used as it is (not Grimoire's: it is never stopped), the Pair sheet finds two controllers (a name
# that is not printable shown safely), the second will not pair, the first does, then is forgotten;
# Bluetooth turned off and on again, and the helper going and coming back with shortcuts.
wait 2000
expect library.bluetooth is external
tap 1535 95
wait 200
tap 746 1930
wait 200
expect library.sheet is shortcuts
tap 930 895
wait 300
expect library.sheet is pair
wait 3000
expect library.pair is idle 2 0
shot 01-found
tap 930 1055
wait 2500
expect toast is Could not pair that controller
expect library.pair is idle 2 0
tap 930 895
wait 300
expect library.pair is pairing 2 0
wait 2500
expect toast is Paired. The controller connects in a moment.
expect library.pair is idle 2 1
shot 02-paired
tap 930 1160
wait 1500
expect toast is Forgotten.
expect library.pair is idle 2 0
tap 930 1375
wait 200
expect library.sheet is shortcuts
tap 930 735
wait 200
expect settings.controller is 0
expect library.bluetooth is stopped
shot 03-bluetooth-off
# On again; then shortcuts off take it away, and back on bring it back
tap 930 735
wait 200
expect library.bluetooth is external
tap 930 575
wait 200
expect settings.shortcuts_enabled is 0
expect library.bluetooth is stopped
tap 930 575
wait 200
expect library.bluetooth is external
quit
