# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/penbutton/button2 .
wait 200
expect pen.gesture is
tap 1535 95
wait 100
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 100
expect library.sheet is gestures
expect library.ui_at.930.749 is gestures.group.editing
tap 930 749
wait 100
expect library.gestures_group is editing
expect library.gestures.1 is Undo: no gesture
expect library.gestures.2 is Redo: no gesture
expect library.ui_at.930.600 is gestures.row.2
tap 930 600
wait 100
expect library.gestures.learning is redo
expect library.record is wait
pen hover 930 1100
side click 2
wait 400
expect library.record is pattern pen.Button.double
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
pen leave
expect library.ui_at.930.2030 is done
tap 930 2030
wait 100
expect library.ui_at.930.2030 is done
tap 930 2030
wait 100
expect library.ui_at.1550.1930 is done
tap 1550 1930
wait 100
expect library.sheet is none
tap 965 95
wait 100
expect library.sheet is new
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.strokes is 0
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 1
# Unsupported Button2 is absent from the row and action probe; its raw line survives the save.
expect gestures.undo is
expect editor.strokes is 1
