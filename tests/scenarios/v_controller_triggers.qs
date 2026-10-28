# settings: shortcuts_enabled=1
# An Xbox-layout controller's analog triggers (8BitDo's Xbox mode through Linux's generic driver:
# ABS_Z and ABS_RZ, 0 to 1023) are buttons, LT and RT: pressed past half their travel, let go at a
# quarter or under, once each way however the hand wavers between. The Buttons sheet learns and
# names them, controller.ini keeps them by name, and the editor acts on them, Erase while held from
# the pull to the let-go.
# rust-only
wait 150
hid attach pad Xbox Bluetooth Gamepad
wait 100
tap 1535 95
wait 200
tap 746 1930
wait 200
tap 930 1215
wait 200
tap 930 629
wait 100
expect library.buttons_group is editing
tap 930 576
wait 100
expect library.learning is undo
axis 2 300
wait 100
expect library.learning is undo
axis 3 65535
axis 0 0
wait 100
expect library.learning is undo
axis 2 700
wait 100
expect library.learning is
expect pads.undo is LT
expect library.buttons.undo is LT
axis 2 400
axis 2 900
axis 2 100
tap 930 1056
wait 100
expect library.learning is hold-eraser
axis 5 1023
wait 100
expect library.learning is
expect pads.hold-eraser is RT
shot 01-triggers-learned
axis 5 0
tap 930 1696
wait 100
tap 930 1589
wait 100
tap 930 1695
wait 100
expect library.sheet is none
# In a notebook: LT undoes once per pull, RT erases while it is held
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen line 300 700 600 720 8 0.6 0.7
wait 80
pen line 300 900 600 920 8 0.6 0.7
wait 80
expect editor.strokes is 2
axis 2 1023
wait 80
expect editor.strokes is 1
axis 2 400
axis 2 1023
wait 80
expect editor.strokes is 1
axis 2 0
axis 2 1023
wait 80
expect editor.strokes is 0
axis 2 0
pen line 300 700 600 720 8 0.6 0.7
wait 80
expect editor.strokes is 1
axis 5 800
wait 60
expect editor.hold_eraser is 1
pen line 450 600 450 800 10 0.6 0.7
wait 100
expect editor.strokes is 0
axis 5 300
wait 60
expect editor.hold_eraser is 1
axis 5 200
wait 60
expect editor.hold_eraser is 0
quit
