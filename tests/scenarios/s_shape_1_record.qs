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
expect library.ui_at.930.789 is gestures.group.editing
tap 930 789
wait 100
expect library.gestures_group is editing
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 100
expect library.gestures.learning is undo
expect library.record is wait
shape Z 930 1150 45 600
expect library.record is shape 1
expect library.record.line is Good. Draw it again (2 of 3).
shape Z 930 1150 45 600
expect library.record is ready
shape Z 930 1150 45 600
expect library.record is ready
shot 01-three-samples
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.shape_count is 1
expect gestures.shape.undo is 3
expect library.gestures.1 has Undo
shot 02-undo-thumbnail
