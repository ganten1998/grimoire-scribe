# root: r_passcode
# env: GRIMOIRE_PASSCODE=enabled
# A subsequent start still uses native unlock and shows the ordinary calibration tip.
wait 300
expect native_pin is 1
expect toast has Calibrate pen
quit
