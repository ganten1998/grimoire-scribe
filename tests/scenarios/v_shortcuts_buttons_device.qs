# settings: status_mode=1
# settings: shortcuts_enabled=1
# Settings > Shortcuts > Buttons learns a press only into the buttons the pressed controller uses (2.0.0-beta.2):
# the sheet shows one device's own buttons, or the default (controller.ini), and a press from a controller that
# uses others is not learned (a toast names it; the row still waits), so a button learned always does on that
# controller what the sheet says. A keyboard's own shortcuts leave the sheet on the default: no controller looks
# in a keyboard's profile. A write that fails names the file it was for. With shortcuts off no device is read,
# so the device row lists none, not a saved one as "not connected".
# rust-only
wait 150
tap 1535 95
wait 100
tap library shortcuts
wait 100
hid attach key:05ac:0256 Magic Keyboard
hid attach pad:2dc8:6006 Pro 2
hid attach pad:2dc8:1003 Arcade Stick
wait 100
expect library.shortcuts.device is magic-keyboard-05ac-0256
# The keyboard's own shortcuts on: Buttons... is still the default's, and the Arcade Stick (no profile) learns
# into controller.ini, which it uses.
tap library shortcuts.own
wait 100
expect library.shortcuts.own is on
expect shortcuts.profiles is magic-keyboard-05ac-0256:on
tap library buttons
wait 100
expect library.sheet is buttons
expect library.buttons.dev is
tap library buttons.group.editing
wait 100
tap library buttons.row.1
wait 100
expect library.learning is undo
hid keys Arcade Stick
key 305 0 1
wait 100
expect library.learning is
expect library.buttons.undo is East
expect pads.undo is East
tap library done
wait 100
tap library done
wait 100
expect library.sheet is shortcuts
# The Pro 2's own shortcuts on (a copy of the default, East for Undo), then the device row back on the
# keyboard: the Buttons sheet is the default's, and the Pro 2's press is not learned into it.
tap library shortcuts.device
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
tap library shortcuts.own
wait 100
expect library.shortcuts.own is on
expect shortcuts.profiles is magic-keyboard-05ac-0256:on pro-2-2dc8-6006:on
tap library shortcuts.device
wait 100
expect library.shortcuts.device is arcade-stick-2dc8-1003
tap library shortcuts.device
wait 100
expect library.shortcuts.device is magic-keyboard-05ac-0256
tap library buttons
wait 100
expect library.buttons.dev is
tap library buttons.group.editing
wait 100
tap library buttons.row.1
wait 100
expect library.learning is undo
hid keys Pro 2
key 304 0 1
wait 100
expect toast is Pro 2 has buttons of its own: choose it in the Device row first.
expect library.learning is undo
expect library.buttons.undo is East
expect pads.undo is East
shot 01-refused
tap library done
wait 100
tap library done
wait 100
# The Pro 2 chosen: its own buttons. The Arcade Stick's press is not learned into them; the Pro 2's is, and the
# default keeps East.
tap library shortcuts.device
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
tap library buttons
wait 100
expect library.buttons.dev is pro-2-2dc8-6006
tap library buttons.group.editing
wait 100
tap library buttons.row.1
wait 100
hid keys Arcade Stick
key 304 0 1
wait 100
expect toast is Arcade Stick uses the default buttons: choose it in the Device row first.
expect library.learning is undo
expect library.buttons.undo is East
hid keys Pro 2
key 304 0 1
wait 100
expect library.learning is
expect library.buttons.undo is South
expect pads.undo is East
# A full storage: the toast names the device's shortcuts, not controller.ini, and nothing changes.
storage full
tap library buttons.row.1
wait 100
key 305 0 1
wait 100
expect toast is Couldn't save the device's shortcuts
expect library.buttons.undo is South
storage ok
tap library done
wait 100
tap library done
wait 100
tap library done
wait 100
expect library.sheet is none
# In a notebook: the Pro 2's South undoes; the Arcade Stick's South does not, its East does.
tap 965 95
wait 100
tap 1564 2254
wait 200
pen line 300 700 600 720 8 0.6 0.7
wait 80
pen leave
pen line 300 900 600 920 8 0.6 0.7
wait 80
pen leave
expect editor.strokes is 2
hid keys Pro 2
key 304 0 1
wait 80
expect editor.strokes is 1
hid keys Arcade Stick
key 304 0 1
wait 80
expect editor.strokes is 1
key 305 0 1
wait 80
expect editor.strokes is 0
# Shortcuts off: none is read, so none is listed; on again, the device chosen is as it was.
pentap panel library
wait 300
tap 1535 95
wait 100
tap library shortcuts
wait 100
expect library.shortcuts.device is pro-2-2dc8-6006
tap library shortcuts.enable
wait 100
expect settings.shortcuts_enabled is 0
expect library.shortcuts.device is
expect library.ui_has.shortcuts.device is 0
expect library.ui_has.shortcuts.own is 0
shot 02-off
tap library shortcuts.enable
wait 100
expect settings.shortcuts_enabled is 1
expect library.shortcuts.device is pro-2-2dc8-6006
quit
