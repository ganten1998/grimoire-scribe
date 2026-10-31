# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures/tap-undo .
wait 200
tap 1535 95
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.editing
wait 100
expect library.gestures_group is editing
expect gestures.undo is touch.2.tap
touch down library gestures.row.1
wait 700
touch up library gestures.row.1
expect gestures.undo is
expect toast is No gestures for Undo now.
tap library done
wait 100
tap library gestures.group.index
wait 100
touch down library gestures.row.2
wait 700
touch up library gestures.row.2
expect gestures.index-map is
tap library done
wait 100
tap library gestures.reset
wait 100
expect gestures.index-map is touch.2.down
expect gestures.undo is
