# settings: shortcuts_enabled=1
# copy: tests/fixtures/shortcuts-1.2 .
# 1.2.0's shortcuts.ini, as a Kindle that had shortcuts on keeps it: it binds no D-pad direction, so
# the D-pad's starter lines come with it (the file is not changed) and the D-pad turns pages and
# tools as with a new file. A Switch-mode controller ("Pro Controller": Linux's generic driver
# numbers its buttons in the Pro Controller's order) has them named in the Buttons sheet as printed
# on it, also one learned there; another controller's keep the files' names.
# rust-only
wait 150
expect pads.previous-page is Select,Left
expect pads.next-page is Start,Right
expect pads.previous-tool is L1,Up
expect pads.next-tool is R1,Down
expect library.buttons.undo is South
# The D-pad in the editor: Right on the last page adds one, Left goes back, Down and Up step the tools
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.pages is 1
key 547 0 1
wait 200
expect editor.pages is 2
expect editor.page is 2
key 546 0 1
wait 200
expect editor.page is 1
key 545 0 1
wait 60
expect editor.tool is mech
key 544 0 1
wait 60
expect editor.tool is pencil
tap panel library
wait 400
expect screen is library
# A Switch-mode controller: its own labels
hid attach pad Pro Controller
wait 100
expect library.buttons.undo is B
expect library.buttons.tool:eraser is A
expect library.buttons.redo is X
expect library.buttons.white is L
expect library.buttons.previous-tool is ZL, Up
expect library.buttons.next-page is RS, Right
expect library.buttons.clear-page is none
tap 1535 95
wait 200
tap 746 1930
wait 200
tap 930 1215
wait 200
tap 930 682
wait 100
expect library.buttons_group is editing
shot 01-switch-editing
tap 930 576
wait 100
expect library.learning is undo
key 306 0 1
wait 100
expect library.learning is
expect pads.undo is 306
expect library.buttons.undo is Y
shot 02-switch-learned
# Another controller: the files' names
hid detach Pro Controller
hid attach pad Xbox Wireless Controller
wait 100
expect library.buttons.undo is 306
expect library.buttons.white is West
shot 03-other-controller
quit
