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
expect library.ui_at.930.1076 is gestures.group.index
tap 930 1076
wait 100
expect library.gestures_group is index
expect library.ui_at.930.296 is gestures.row.1
tap 930 296
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
side click 2
wait 400
expect pen.gesture is Button.double
expect editor.bar is open
side click 2
wait 400
expect editor.bar is 0
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 1
# Triple-click has no built-in Clear page action, even with a custom double-click binding.
# Each release-to-next-press gap is 150 ms, below the 350 ms limit.
side click 3
wait 400
expect pen.gesture is Button.triple
expect editor.strokes is 1
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 2
# A hovering hold has no built-in action either (2.1.0-beta.2: it cleared the page, issue #39): the ink stays.
pen hover 930 1100
side on
wait 1300
expect pen.gesture is Button.hold
expect editor.strokes is 2
expect toast is
side off
expect editor.hold_eraser is 0
