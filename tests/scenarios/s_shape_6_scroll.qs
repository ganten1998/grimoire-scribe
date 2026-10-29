# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: landscape_fit=0
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
expect library.ui_at.930.1616 is done
tap 930 1616
wait 100
expect library.ui_at.930.1109 is gestures.group.pages
tap 930 1109
wait 100
expect library.ui_at.930.1376 is gestures.row.7
tap 930 1376
wait 100
shape Z 930 1150 45 600
shape Z 930 1150 45 600
expect library.ui_at.500.1650 is record.use
tap 500 1650
wait 100
expect library.record is 0
expect gestures.shape.pages is 2
expect library.ui_at.930.1856 is done
tap 930 1856
wait 100
expect library.ui_at.930.2069 is done
tap 930 2069
wait 100
expect library.sheet is settings
expect library.ui_at.1550.1930 is done
tap 1550 1930
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
rotate 1
wait 1200
pinch 1240 900 300 700 6
wait 350
expect editor.zoom not 1.00
swipe 2 1240 900 0 -400
expect touch.gesture is
expect editor.modal is none
shape Z 1100 950 45 600
expect editor.modal is pages
expect fshape.fired is 1
