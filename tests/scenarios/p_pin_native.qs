# settings: status_mode=1
# env: GRIMOIRE_PASSCODE=enabled
# Native PIN keeps ownership of input until outOfScreenSaver. Time and touches never unlock.
wait 200
expect native_pin is 1
expect input.yielded is 0
sleep
wait 200
expect asleep is 1
expect asleep.mode is 2
expect input.yielded is 1
powerd active
power
wait 3300
expect asleep is 1
expect input.yielded is 1
tap 965 95
wait 100
expect screen is library
power
wait 3300
expect running is 1
expect asleep is 1
wake
wait 200
expect asleep is 0
expect input.yielded is 0
expect screen is library
shot native-pin-resumed
# Sleep in the middle of a stroke: the native keypad will consume its lift.
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 200
pen down 300 700 0.6
pen move 500 750 0.7
sleep
wait 100
expect asleep is 1
expect editor.strokes is 1
wake
wait 100
pen line 500 900 900 950 8 0.6 0.7
wait 100
expect editor.strokes is 2
quit
