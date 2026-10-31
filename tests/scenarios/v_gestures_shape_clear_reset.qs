# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# root: s_shape_1_record
wait 200
tap 1535 95
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.editing
wait 100
expect library.gestures_group is editing
expect gestures.shape.undo is 3
touch down library gestures.row.1
wait 700
touch up library gestures.row.1
expect gestures.shape_count is 0
expect toast is No gestures for Undo now.
tap library gestures.row.1
wait 100
expect library.gestures.learning is undo
expect library.record is wait
shape Z 930 1150 45 600
expect library.record is shape 1
shape Z 930 1150 45 600
expect library.record is ready
shape Z 930 1150 45 600
expect library.record is ready
tap library record.use
wait 100
expect library.record is 0
expect gestures.shape_count is 1
tap library done
wait 100
tap library gestures.reset
wait 100
expect gestures.shape_count is 1
tap library gestures.reset.cancel
wait 100
expect gestures.shape_count is 1
tap 930 1909
wait 100
tap library gestures.reset.confirm
wait 100
expect gestures.shape_count is 0
expect gestures.index-map is touch.2.down
