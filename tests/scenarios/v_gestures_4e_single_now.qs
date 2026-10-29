# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/penbutton/single-now .
wait 200
expect gestures.undo is pen.Button.tap
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
side click 1
expect pen.gesture is Button.tap
expect pen.clicks is 0
expect editor.strokes is 0
