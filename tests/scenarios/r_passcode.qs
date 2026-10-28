# settings: status_mode=1
# env: GRIMOIRE_PASSCODE=enabled
# Native PIN is retained. There is no old "passcode paused" notice.
wait 300
expect screen is library
expect native_pin is 1
expect toast not passcode is paused
quit
