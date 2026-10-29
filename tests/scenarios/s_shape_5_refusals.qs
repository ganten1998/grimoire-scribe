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
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 100
expect library.gestures.learning is undo
expect library.record is wait
shape V 930 1150 45 600
expect library.record.line has Too easy to make by accident.
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 100
expect library.record is wait
shape Z 930 1150 8 600
expect library.record.line is Draw it bigger: at least 2 cm across.
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 100
expect library.record is wait
shape Z 930 1150 45 600
expect library.record is shape 1
shape S 930 1150 45 600
expect library.record.line is Not like the first one: draw it again.
expect library.record is shape 1
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 100
expect library.record is wait
fpath 600 1100 1200 1100 200
fpath 600 1100 1200 1100 200
expect library.record.line is One stroke, without lifting.
expect library.ui_at.1360.1650 is record.cancel
tap 1360 1650
wait 100
expect library.ui_at.930.816 is gestures.row.2
tap 930 816
wait 100
shape Z 930 1150 45 600
shape Z 930 1150 45 600
shape Z 930 1150 45 600
expect library.record.line is That is the shape for Undo. Use moves it here.
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.shape.redo is 3
expect gestures.shape.undo is
expect toast is That shape is no longer for Undo.
# A skewed near-Z is consistent but too close to the stored Redo shape.
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 100
fpath 600 900 1200 900 840 1500 1200 1500 600
fpath 600 900 1200 900 840 1500 1200 1500 600
fpath 600 900 1200 900 840 1500 1200 1500 600

expect library.record.line is Too close to the shape for Redo.
