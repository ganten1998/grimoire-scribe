# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: side_button=2
wait 200
expect settings.side_button is 2
tap 965 95
wait 100
expect library.sheet is new
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.strokes is 0
side on
pen down 300 700 0.6
pen move 600 700 0.6
expect display.ink is 1
expect display.pen_mode is 1
pen move 1300 700 0.6
pen up 1300 700
side off
expect pen.gesture is Button.drag
expect editor.strokes is 1
expect editor.stroke_look is highlighter
expect display.ink is 0
