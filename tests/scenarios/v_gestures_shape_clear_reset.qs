# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# root: s_shape_1_record
wait 200
tap 1535 95
wait 100
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 100
expect library.sheet is gestures
expect library.ui_at.930.789 is gestures.group.editing
tap 930 789
wait 100
expect library.gestures_group is editing
expect gestures.shape.undo is 3
expect library.ui_at.930.656 is gestures.row.1
touch down 930 656
wait 700
touch up 930 656
expect gestures.shape_count is 0
expect toast is No gestures for Undo now.
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 100
expect library.gestures.learning is undo
expect library.record is wait
shape Z 930 1150 45 600
expect library.record is shape 1
shape Z 930 1150 45 600
expect library.record is ready
shape Z 930 1150 45 600
expect library.record is ready
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.shape_count is 1
expect library.ui_at.930.1616 is done
tap 930 1616
wait 100
expect library.ui_at.930.1909 is gestures.reset
tap 930 1909
wait 100
expect gestures.shape_count is 1
expect library.ui_at.1200.1120 is gestures.reset.cancel
tap 1200 1120
wait 100
expect gestures.shape_count is 1
tap 930 1909
wait 100
expect library.ui_at.650.1120 is gestures.reset.confirm
tap 650 1120
wait 100
expect gestures.shape_count is 0
expect gestures.pages is touch.2.down
