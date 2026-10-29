# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures/tap-undo .
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
expect gestures.undo is touch.2.tap
expect library.ui_at.930.656 is gestures.row.1
touch down 930 656
wait 700
touch up 930 656
expect gestures.undo is
expect toast is No gestures for Undo now.
expect library.ui_at.930.1616 is done
tap 930 1616
wait 100
expect library.ui_at.930.1109 is gestures.group.pages
tap 930 1109
wait 100
expect library.ui_at.930.1376 is gestures.row.7
touch down 930 1376
wait 700
touch up 930 1376
expect gestures.pages is
expect library.ui_at.930.1856 is done
tap 930 1856
wait 100
expect library.ui_at.930.1909 is gestures.reset
tap 930 1909
wait 100
expect gestures.pages is touch.2.down
expect gestures.undo is
