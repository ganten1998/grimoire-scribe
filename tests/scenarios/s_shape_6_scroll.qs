# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: landscape_fit=0
wait 200
tap 1535 95
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.editing
wait 100
expect library.gestures_group is editing
tap library done
wait 100
tap library gestures.group.pages
wait 100
tap library gestures.row.7
wait 100
shape Z 930 1150 45 600
shape Z 930 1150 45 600
tap library record.use
wait 100
expect library.record is 0
expect gestures.shape.pages is 2
tap library done
wait 100
tap library done
wait 100
expect library.sheet is settings
tap library done
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
