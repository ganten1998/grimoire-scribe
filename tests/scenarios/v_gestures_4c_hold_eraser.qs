# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
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
expect library.ui_at.930.920 is gestures.row.4
tap 930 920
wait 100
expect library.gestures.learning is hold-eraser
expect library.record is wait
pen hover 930 1100
side click 1
wait 400
expect library.record is wait
expect library.record.line is Hold eraser needs the button held: hold it or drag with it.
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
side on
wait 1300
side off
expect library.record is pattern pen.Button.hold
# Hold is already the built-in Clear page slot; Use explicitly moves it.
expect library.record.line is Now for Clear page. Use moves it here.
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
expect gestures.hold-eraser is pen.Button.hold
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
