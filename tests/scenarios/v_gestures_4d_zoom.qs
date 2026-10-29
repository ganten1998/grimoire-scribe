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
expect library.ui_at.930.1069 is gestures.group.pages
tap 930 1069
wait 100
expect library.gestures_group is pages
expect library.ui_at.930.1960 is gestures.row.11
tap 930 1960
wait 100
expect library.gestures.learning is zoom
expect library.record is wait
# These rows accept only a pen-button drag, including when the touch recognizer succeeds.
tapn 2 930 1100
expect library.record is wait
expect library.record.line is Hold the pen button and drag with it.
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
pen hover 930 1100
side click 1
wait 400
expect library.record is wait
expect library.record.line is Hold the pen button and drag with it.
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
side on
expect display.presents not -1
pen down 800 1100 0.6
pen move 1000 1200 0.6
# Recording defers its UI refresh until lift; button drag cannot click the card or draw.
expect display.presents is
expect display.ink is 0
pen up 1000 1200
side off
expect library.record is pattern pen.Button.drag
expect library.record.line is pen button drag
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
expect gestures.zoom is pen.Button.drag
pen leave
expect library.ui_at.930.2120 is done
tap 930 2120
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
expect editor.zoom is 1.00
pen hover 800 1100
side on
pen down 800 1100 0.6
expect pen.gesture is Button.drag
expect display.ink is 0
pen move 600 800 0.6
expect display.ink is 0
expect display.pen_mode is 0
pen up 600 800
side off
expect editor.strokes is 0
expect editor.zoom not 1.00
expect editor.zoomed is 1
