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
expect library.ui_at.930.1229 is gestures.group.index
tap 930 1229
wait 100
expect library.gestures_group is index
expect library.ui_at.930.520 is gestures.row.1
tap 930 520
wait 100
expect library.gestures.learning is links
expect library.record is wait
pen hover 930 1100
side click 2
wait 400
expect library.record is pattern pen.Button.double
expect library.record.line is pen button double-click
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
# The action probe prefers the default touch source; the harness also verifies the saved pen line.
expect gestures.links is touch.2.up
expect library.gestures.1 is Links on this page: 2 fingers up +1
pen leave
expect library.ui_at.930.1790 is done
tap 930 1790
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
side click 2
wait 400
expect pen.gesture is Button.double
expect editor.bar is open
side click 2
wait 400
expect editor.bar is 0
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 1
# Each release-to-next-press gap is 150 ms, below the new 350 ms limit.
side click 3
wait 400
expect pen.gesture is Button.triple
expect editor.strokes is 0
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 1
pen hover 930 1100
side on
wait 1300
expect pen.gesture is Button.hold
expect editor.strokes is 0
side off
expect editor.hold_eraser is 0
