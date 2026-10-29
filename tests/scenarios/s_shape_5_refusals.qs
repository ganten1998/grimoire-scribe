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
tap library gestures.row.1
wait 100
expect library.gestures.learning is undo
expect library.record is wait
shape V 930 1150 45 600
expect library.record.line has Too easy to make by accident.
tap library record.retry
wait 100
expect library.record is wait
shape Z 930 1150 8 600
expect library.record.line is Draw it bigger: at least 2 cm across.
tap library record.retry
wait 100
expect library.record is wait
shape Z 930 1150 45 600
expect library.record is shape 1
shape S 930 1150 45 600
expect library.record.line is Not like the first one: draw it again.
expect library.record is shape 1
tap library record.retry
wait 100
expect library.record is wait
fpath 600 1100 1200 1100 200
fpath 600 1100 1200 1100 200
expect library.record.line is One stroke, without lifting.
tap library record.cancel
wait 100
tap library gestures.row.2
wait 100
shape Z 930 1150 45 600
shape Z 930 1150 45 600
shape Z 930 1150 45 600
expect library.record.line is That is the shape for Undo. Use moves it here.
tap library record.use
wait 100
expect library.record is 0
expect gestures.shape.redo is 3
expect gestures.shape.undo is
expect toast is That shape is no longer for Undo.
# A skewed near-Z is consistent but too close to the stored Redo shape.
tap library gestures.row.1
wait 100
fpath 600 900 1200 900 840 1500 1200 1500 600
fpath 600 900 1200 900 840 1500 1200 1500 600
fpath 600 900 1200 900 840 1500 1200 1500 600

expect library.record.line is Too close to the shape for Redo.
