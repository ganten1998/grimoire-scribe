# settings: status_mode=1
# settings: shortcuts_enabled=1
# rust-only
# The size keys' row (2.2), "Smaller / larger size", last of the Tools group's twelve on both pages of the Settings
# menu: Buttons learns a controller's two buttons for it, the smaller size's first, as every paired row does, and
# those buttons then step the pen in use on a page (a toast says the size); Gestures lists it with its gesture.
wait 200
tap library settings
wait 200
tap library settings.cat.4
wait 200
tap library shortcuts
wait 200
expect library.sheet is shortcuts
tap library buttons
wait 200
expect library.sheet is buttons
tap library buttons.group.tools
wait 200
expect library.buttons_group is tools
expect library.ui_has.buttons.row.12 is 1
expect library.ui_has.buttons.row.13 is 0
shot 01-buttons-tools
tap library buttons.row.12
wait 100
expect library.learning is size-smaller
key 312 0 1
wait 100
expect library.learning is size-larger
key 313 0 1
wait 100
expect library.learning is
expect pads.size-smaller is L2
expect pads.size-larger is R2
shot 02-buttons-tools-learned
# Gestures > Tools: the row with its gesture (none built in)
key 1 0
wait 200
key 1 0
wait 200
key 1 0
wait 200
tap library gestures
wait 200
expect library.sheet is gestures
tap library gestures.group.tools
wait 200
expect library.gestures_group is tools
expect library.gestures.12 is Smaller / larger size: no gesture
expect library.ui_has.gestures.row.12 is 1
shot 03-gestures-tools
# On a page: R2 and L2 step the pencil's size
key 1 0
wait 200
key 1 0
wait 200
key 1 0
wait 300
expect library.sheet is none
key 49 1
wait 100
expect library.sheet is new
key 28 0
wait 300
expect screen is editor
expect editor.tool is pencil
key 313 0 1
wait 100
expect settings.pencil.size_max_mm is 0.5000
expect toast is Size: 0.5 mm
key 312 0 1
wait 100
key 312 0 1
wait 100
expect settings.pencil.size_max_mm is 0.2500
expect toast is Size: 0.25 mm
