# No-margin paper, highlighter on the pen button, clear gestures, eraser end, pinch zoom, rotation
wait 300
tap 965 95
wait 100
tap 1346 460
wait 50
shot 01-new-sheet-8-papers
tap 1564 2254
wait 400
pen curve 300 520 40 70 0.7 5
pen curve 300 660 40 70 0.7 5
wait 100
# hold the pen button while drawing = highlighter over the writing
side on
pen line 280 540 1200 540 60 0.5 0.5
side off
wait 400
shot 02-highlighter-no-margin
# eraser end removes the second line of writing
eraser line 700 600 720 720 15
wait 300
shot 03-eraser-end
# triple click clears, undo brings it back
side on
side off
side on
side off
side on
side off
wait 400
shot 04-cleared
pentap 1768 594
wait 400
shot 05-undo-clear
# pinch zoom in around the writing, write small, then reset with the chip
pinch 600 600 300 750 6
wait 300
shot 06-zoomed
pen curve 500 900 20 60 0.6 4
wait 300
shot 07-zoomed-writing
tap 930 74
wait 300
shot 08-zoom-reset
# turn the Kindle sideways and back
rotate 1
wait 600
shot 09-landscape
pen line 300 1200 1500 1250 40 0.6 0.6
wait 300
shot 10-landscape-writing
rotate 0
wait 600
shot 11-portrait-again
quit
