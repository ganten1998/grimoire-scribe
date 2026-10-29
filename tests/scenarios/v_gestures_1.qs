# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
wait 200
tap 1535 95
wait 100
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 100
expect library.sheet is gestures
expect library.ui_at.930.1109 is gestures.group.pages
tap 930 1109
wait 100
expect library.gestures_group is pages
expect library.ui_at.930.1376 is gestures.row.7
tap 930 1376
wait 100
expect library.gestures.learning is pages
swipe 2 930 1100 0 400
expect library.record is pattern touch.2.down
expect library.record.line is 2 fingers down
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.pages is touch.2.down
expect library.ui_at.930.1856 is done
tap 930 1856
wait 100
expect library.ui_at.930.789 is gestures.group.editing
tap 930 789
wait 100
expect library.gestures_group is editing
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 100
expect library.gestures.learning is undo
expect library.record is wait
tapn 2 930 1100
expect library.record is pattern touch.2.tap
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.undo is touch.2.tap
expect library.ui_at.930.816 is gestures.row.2
tap 930 816
wait 100
tapn 2 930 1100
expect library.record.line is Now for Undo. Use moves it here.
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.redo is touch.2.tap
expect gestures.undo is
expect toast is That gesture is no longer for Undo.
