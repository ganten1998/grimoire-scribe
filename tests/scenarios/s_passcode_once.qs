# root: r_passcode
# env: GRIMOIRE_PASSCODE=enabled
# A subsequent start still uses native unlock without a calibration startup tip.
wait 300
expect native_pin is 1
expect toast is
quit
