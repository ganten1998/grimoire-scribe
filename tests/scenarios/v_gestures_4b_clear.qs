# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
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
expect library.gestures.7 is Clear page: pen button triple-click
expect library.ui_at.930.1400 is gestures.row.7
touch down 930 1400
wait 700
touch up 930 1400
expect gestures.clear-page is
expect library.gestures.7 is Clear page: no gesture
expect toast is No gestures for Clear page now.
expect library.ui_at.930.2030 is done
tap 930 2030
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
side click 3
wait 400
expect editor.strokes is 1
pen hover 930 1100
side on
wait 1300
side off
expect editor.strokes is 1
