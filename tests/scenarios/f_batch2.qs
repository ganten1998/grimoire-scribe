# No-margin paper, highlighter on the pen button, clear gestures, eraser end, pinch zoom, rotation
wait 300
tap 965 95
wait 100
tap 1346 460
wait 50
expect library.sheet is new
expect library.paper is 6
shot 01-new-sheet
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 6
pen curve 300 520 40 70 0.7 5
pen curve 300 660 40 70 0.7 5
wait 100
expect editor.strokes is 2
# hold the pen button while drawing = highlighter over the writing
side on
pen line 280 540 1200 540 60 0.5 0.5
side off
wait 400
shot 02-highlighter-no-margin
expect editor.strokes is 3
expect editor.tool is pencil
# eraser end removes the second line of writing
eraser line 700 600 720 720 15
wait 300
shot 03-eraser-end
# (its end also reaches the highlighter band's lower edge: only the first line is left)
expect editor.strokes is 1
# triple click clears, undo brings it back
side on
side off
side on
side off
side on
side off
wait 400
expect toast is Page cleared
shot 04-cleared
expect editor.strokes is 0
pentap 1768 594
wait 400
shot 05-undo-clear
expect editor.strokes is 1
# pinch zoom in around the writing, write small, then reset with the chip
pinch 600 600 300 750 6
wait 300
shot 06-zoomed
expect editor.zoom not 1.00
pen curve 500 900 20 60 0.6 4
wait 300
shot 07-zoomed-writing
expect editor.strokes is 2
tap 930 74
wait 300
shot 08-zoom-reset
expect editor.zoom is 1.00
# turn the Kindle sideways and back
rotate 1
wait 600
shot 09-landscape
expect rotation is 1
expect size is 2480x1860
expect editor.zoom is 1.00
pen line 300 1200 1500 1250 40 0.6 0.6
wait 300
shot 10-landscape-writing
expect editor.strokes is 3
rotate 0
wait 600
shot 11-portrait-again
expect rotation is 0
expect size is 1860x2480
expect editor.strokes is 3
quit
