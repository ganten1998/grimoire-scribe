# settings: status_mode=1
# env: GRIMOIRE_SIM_NO_SERVICES=wifi,updates
# Settings on a platform that leaves two device services to its system: their two buttons are gone,
# the others stay, and each of those still opens its sheet.
# rust-only
wait 300
tap library settings
wait 300
expect library.sheet is settings
expect library.ui_has.calibrate is 1
expect library.ui_has.ko is 1
expect library.ui_has.pair-menu is 1
expect library.ui_has.wifi is 0
expect library.ui_has.updates is 0
expect library.ui_has.gestures is 1
expect library.ui_has.done is 1
shot 01-settings-without-wifi-and-updates
tap library pair-menu
wait 300
expect library.sheet is pair-menu
quit
