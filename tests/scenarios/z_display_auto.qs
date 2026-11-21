# rust-only
# env: GRIMOIRE_SIM_LIGHT_SENSOR=1 GRIMOIRE_CHECK_FIT=1
# The Display page's Auto brightness row (issue #137): the Kindle's own ambient-light auto
# brightness (com.lab126.powerd flAuto) as a checkbox under Brightness. Only a Kindle with a light
# sensor has the row; GRIMOIRE_SIM_LIGHT_SENSOR gives the simulator one. The light itself is the
# power service's, which follows its sensor while the row is ticked.
wait 300
tap library add
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pentap panel display
wait 300
expect editor.modal is display
expect light.auto is 0
expect light.intensity is 12
tap panel display.auto
wait 300
expect light.auto is 1
# The Brightness slider follows the level the power service picked for the room.
expect light.intensity is 18
shot 01-auto-brightness-on
tap panel display.auto
wait 300
expect light.auto is 0
# The Brightness slider under it still drags (the row only takes its own tap), and Done closes.
drag touch 1000 296 1500 296 5
wait 300
expect light.intensity not 12
tap panel display.done
wait 300
expect editor.modal is none
quit
