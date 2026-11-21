# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
wait 200
expect pen.gesture is
tap library settings
wait 100
expect library.ui_at.329.830 is settings.cat.4
tap 329 830
wait 100
expect library.ui_at.1233.639 is gestures
tap 1233 639
wait 100
expect library.sheet is gestures
expect library.ui_at.930.608 is gestures.group.editing
tap 930 608
wait 100
expect library.gestures_group is editing
expect library.ui_at.930.758 is gestures.row.4
tap 930 758
wait 100
expect library.gestures.learning is hold-eraser
expect library.record is wait
pen hover 930 1100
side click 1
wait 400
expect library.record is wait
expect library.record.line is Erase while held needs the button held: hold it or drag with it.
expect library.ui_at.763.1650 is record.retry
pentap 763 1650
wait 100
side on
wait 1300
side off
expect library.record is pattern pen.Button.hold
# The hold has no built-in action (2.1.0-beta.2: it was Clear page): nothing to move.
expect library.record.line is pen button hold
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
expect gestures.hold-eraser is pen.Button.hold
pen leave
expect library.ui_at.125.95 is done
tap 125 95
wait 100
expect library.ui_at.125.95 is settings.back
tap 125 95
wait 100
expect library.ui_at.1735.95 is done
tap 1735 95
wait 100
expect library.sheet is none
tap library add
tap library add.new-notebook
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
pen hover 500 700
side on
# No 1.2-second wait: the eraser starts on the button's press.
expect editor.hold_eraser is 1
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 0
expect editor.hold_eraser is 1
side off
expect editor.hold_eraser is 0
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 1
