# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
wait 200
tap 1535 95
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.pages
wait 100
expect library.gestures_group is pages
tap library gestures.row.7
wait 100
expect library.gestures.learning is pages
swipe 2 930 1100 0 400
expect library.record is pattern touch.2.down
expect library.record.line is 2 fingers down
tap library record.use
wait 100
expect library.record is 0
expect gestures.pages is touch.2.down
tap library done
wait 100
tap library gestures.group.editing
wait 100
expect library.gestures_group is editing
tap library gestures.row.1
wait 100
expect library.gestures.learning is undo
expect library.record is wait
tapn 2 930 1100
expect library.record is pattern touch.2.tap
tap library record.use
wait 100
expect library.record is 0
expect gestures.undo is touch.2.tap
tap library gestures.row.2
wait 100
tapn 2 930 1100
expect library.record.line is Now for Undo. Use moves it here.
tap library record.use
wait 100
expect library.record is 0
expect gestures.redo is touch.2.tap
expect gestures.undo is
expect toast is That gesture is no longer for Undo.
