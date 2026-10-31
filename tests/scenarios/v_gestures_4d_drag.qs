# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/penbutton/pages .
wait 200
expect gestures.index-map is touch.2.down
expect gestures.pages is pen.Button.drag
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
pen hover 500 1000
side on
pen down 500 1000 0.6
expect pen.gesture is Button.drag
expect display.ink is 0
expect display.pen_mode is 0
expect editor.modal is pages
pen move 900 1200 0.6
expect display.ink is 0
pen up 900 1200
side off
wait 100
expect editor.modal is pages
expect editor.strokes is 1
expect display.ink is 0
