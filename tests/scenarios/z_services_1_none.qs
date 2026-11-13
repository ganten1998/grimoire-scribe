# settings: status_mode=1
# env: GRIMOIRE_SIM_NO_SERVICES=wifi,pairing,updates,koreader,calibration
# Settings on a platform that leaves every device service to its system (light.rs, Services): no
# Calibrate row and no KOReader, Pair, Wi-Fi or Updates button; the rest is there and works.
# rust-only
wait 300
tap library settings
wait 300
expect library.sheet is settings
expect library.ui_has.calibrate is 0
expect library.ui_has.ko is 0
expect library.ui_has.pair-menu is 0
expect library.ui_has.wifi is 0
expect library.ui_has.updates is 0
expect library.ui_has.display is 1
expect library.ui_has.pen-feel is 1
expect library.ui_has.shortcuts is 1
expect library.ui_has.gestures is 1
expect library.ui_has.done is 1
shot 01-settings-without-services
tap library gestures
wait 300
expect library.sheet is gestures
quit
