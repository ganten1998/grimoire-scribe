# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# Clear page and the pen button's hold (issues #36, #39). Since 2.1.0-beta.2 no gesture is built in for it: its
# row says "no gesture" and a hold over the page leaves the ink alone. Recorded, the hold clears the page and a
# toast says so and points at Undo, which brings the ink back in one step. Holding the row clears its gestures.
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
expect library.gestures.7 is Clear page: no gesture
expect gestures.clear-page is
# Record a hold of the pen's button for it
expect library.ui_at.930.1320 is gestures.row.7
tap 930 1320
wait 100
expect library.gestures.learning is clear-page
expect library.record is wait
pen hover 930 1100
side on
wait 1300
side off
expect library.record is pattern pen.Button.hold
expect library.record.line is pen button hold
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
expect gestures.clear-page is pen.Button.hold
expect library.gestures.7 is Clear page: pen button hold
pen leave
# Holding the row clears it, and the record card records it again
touch down 930 1320
wait 700
touch up 930 1320
expect gestures.clear-page is
expect library.gestures.7 is Clear page: no gesture
expect toast is No gestures for Clear page now.
tap 930 1320
wait 100
expect library.gestures.learning is clear-page
pen hover 930 1100
side on
wait 1300
side off
expect library.record is pattern pen.Button.hold
pentap 500 1650
wait 100
expect library.record is 0
expect gestures.clear-page is pen.Button.hold
pen leave
expect library.ui_at.930.2120 is done
tap 930 2120
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
# Three clicks have no action
side click 3
wait 400
expect editor.strokes is 1
# The hold clears the page, and says so
pen hover 930 1100
side on
wait 1300
side off
expect pen.gesture is Button.hold
expect editor.strokes is 0
expect toast is Page cleared. Undo brings it back.
shot 01-cleared-toast
pen leave
wait 100
# Undo brings it back, in one step
pentap panel undo
wait 300
expect editor.strokes is 1
expect editor.undo_kind is add
