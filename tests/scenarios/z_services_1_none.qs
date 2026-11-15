# settings: status_mode=1
# env: GRIMOIRE_SIM_NO_SERVICES=wifi,pairing,updates,koreader,calibration GRIMOIRE_SIM_NO_LIGHT=1 GRIMOIRE_CHECK_FIT=1
# Settings on a platform that leaves every device service to its system (light.rs, Services) and has
# no front light: no Calibrate row and no KOReader, Pair, Wi-Fi or Updates row, and the Display row
# is called Display; the rest is there and works. The Display sheet says there is no light to set.
# rust-only
wait 300
tap library settings
wait 300
expect library.sheet is settings
# General: the two services a platform leaves out have no row.
expect library.ui_has.updates is 0
expect library.ui_has.wifi is 0
# Pen: no Calibrate row; the rest of the category is there.
tap library settings.cat.3
wait 100
expect library.ui_has.calibrate is 0
expect library.ui_has.pen-feel is 1
# Controls: no Pair row; Shortcuts and Gestures stay.
tap library settings.cat.4
wait 100
expect library.ui_has.pair-menu is 0
expect library.ui_has.shortcuts is 1
expect library.ui_has.gestures is 1
# Display: its row is still there, under the category's name.
tap library settings.cat.1
wait 100
expect library.ui_has.display is 1
expect library.ui_has.done is 1
shot 01-settings-without-services
tap library display
wait 300
shot 02-display-without-a-light
quit
