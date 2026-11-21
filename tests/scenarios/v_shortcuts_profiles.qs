# settings: status_mode=1
# settings: shortcuts_enabled=1
# Settings > Shortcuts and Settings > Pair (2.0.0-beta.2): Settings' rows in their categories since the new Settings
# menu (#115: Display under Display, Tool Settings and Calibrate under Pen, Shortcuts, Pair and Gestures under
# Controls), upright and sideways. The Shortcuts sheet's devices and one device's own shortcuts: on from a copy of the default, a
# button learned into them (controller.ini never made), that device's presses using them while another's and a
# key from no device use the default; off (the file kept, the default again), on again (as it was), and the
# device listed while it is not connected.
# rust-only
wait 150
tap library settings
wait 100
expect library.sheet is settings
tap library settings.cat.1
wait 100
expect library.ui_at.1233.371 is display
tap library settings.cat.3
wait 100
expect library.ui_at.1233.371 is pen-feel
expect library.ui_at.1233.773 is calibrate
tap library settings.cat.4
wait 100
expect library.ui_at.1233.371 is shortcuts
expect library.ui_at.1233.505 is pair-menu
expect library.ui_at.1233.639 is gestures
rotate 1
wait 200
expect library.sheet is settings
expect library.ui_at.1544.371 is shortcuts
tap library settings.cat.1
wait 100
expect library.ui_at.1544.371 is display
tap library settings.cat.3
wait 100
expect library.ui_at.1544.371 is pen-feel
expect library.ui_at.1544.773 is calibrate
tap library settings.cat.4
wait 100
rotate 0
wait 200
# Pair: no helper here, so its rows are greyed (no control to tap).
tap library pair-menu
wait 100
expect library.sheet is pair-menu
expect library.ui_has.bluetooth is 0
expect library.ui_has.pair is 0
expect library.ui_has.settings.back is 1
key 1 0
wait 100
expect library.sheet is settings
tap library shortcuts
wait 100
expect library.sheet is shortcuts
expect library.shortcuts.device is
expect library.shortcuts.own is none
expect library.ui_has.shortcuts.own is 0
hid attach pad:2dc8:6006 Pro 2
hid attach pad:2dc8:1003 Arcade Stick
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
expect library.shortcuts.own is none
tap library shortcuts.device
wait 100
expect library.shortcuts.device is arcade-stick-2dc8-1003
tap library shortcuts.device
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
tap library shortcuts.own
wait 100
expect library.shortcuts.own is on
expect shortcuts.profiles is pro-2-2dc8-6006:on
shot 01-own-shortcuts
# Learned into its own buttons: East is Undo for it; the default (shortcuts.ini, no controller.ini) keeps South
# for Undo and East for the eraser.
tap library buttons
wait 100
expect library.sheet is buttons
expect library.ui_has.buttons.reset is 0
tap library buttons.group.editing
wait 100
tap library buttons.row.1
wait 100
expect library.learning is undo
hid keys Pro 2
key 305 0 1
wait 100
expect library.learning is
expect library.buttons.undo is East
expect pads.undo is South
expect pads.tool:eraser is East
tap library done
wait 100
expect library.ui_has.buttons.reset is 1
tap library settings.back
wait 100
expect library.sheet is shortcuts
tap library settings.close
wait 100
expect library.sheet is none
# In a notebook: the Pro 2's East undoes, the Arcade Stick's East and a key from no device pick the eraser.
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 200
pen line 300 700 600 720 8 0.6 0.7
wait 80
pen leave
expect editor.strokes is 1
expect editor.tool is pencil
hid keys Arcade Stick
key 305 0 1
wait 80
expect editor.tool is eraser
expect editor.strokes is 1
key 305 0 1
wait 80
expect editor.tool is pencil
hid keys Pro 2
key 305 0 1
wait 80
expect editor.strokes is 0
expect editor.tool is pencil
hid keys
key 305 0 1
wait 80
expect editor.tool is eraser
key 305 0 1
wait 80
expect editor.tool is pencil
# Its own shortcuts off: the default for it too, the file kept.
pentap panel library
wait 300
tap library settings
wait 100
tap library settings.cat.4
wait 100
tap library shortcuts
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
tap library shortcuts.own
wait 100
expect library.shortcuts.own is off
expect shortcuts.profiles is pro-2-2dc8-6006:off
tap library buttons
wait 100
expect library.ui_has.buttons.reset is 0
expect library.buttons.undo is South
tap library settings.back
wait 100
# Gone, it is listed as not connected; on again, its own buttons as they were.
hid detach Pro 2
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
shot 02-not-connected
tap library shortcuts.own
wait 100
expect library.shortcuts.own is on
expect shortcuts.profiles is pro-2-2dc8-6006:on
tap library buttons
wait 100
expect library.buttons.undo is East
tap library settings.back
wait 100
tap library shortcuts.device
wait 100
expect library.shortcuts.device is arcade-stick-2dc8-1003
expect library.shortcuts.own is none
tap library buttons
wait 100
expect library.buttons.undo is South
quit
