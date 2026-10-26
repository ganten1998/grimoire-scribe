# settings: shortcuts_enabled=1
# Controllers as express keys: one another app holds, one coming and going (a toast each way, at
# most one a direction in 30 s, so a link that keeps dropping does not flash the screen), the
# Controller sheet listing what is connected, and the Buttons sheet learning buttons into
# controller.ini (one button per action, Pages in two presses, 10 s without a press gives up, Reset
# goes back to shortcuts.ini's), which the editor then uses.
wait 150
hid attach held Mapper Pad
wait 100
expect toast is Controller held by another app
hid detach Mapper Pad
wait 100
expect toast is Controller disconnected
hid attach pad Pro Controller
wait 100
expect toast is Controller disconnected
tap 1535 95
wait 200
tap 945 1930
wait 200
expect library.sheet is shortcuts
shot 01-controller-sheet
tap 930 1215
wait 200
expect library.sheet is buttons
shot 02-buttons
tap 930 575
wait 100
expect library.learning is undo
shot 03-learning
key 305 0 1
wait 100
expect library.learning is
expect pads.undo is East
expect pads.tool:eraser is
tap 930 1215
wait 100
expect library.learning is previous-page
key 2 0
wait 100
expect library.learning is previous-page
key 316 0 1
wait 100
expect library.learning is next-page
key 317 0 1
wait 100
expect library.learning is
expect pads.previous-page is 316
expect pads.next-page is 317
tap 930 735
wait 100
expect library.learning is redo
wait 10300
expect library.learning is
expect pads.redo is North
shot 04-learned
tap 930 1695
wait 100
expect library.sheet is shortcuts
tap 930 1695
wait 100
expect library.sheet is none
# In a notebook the learned buttons act
tap 965 95
wait 100
tap 1564 2254
wait 200
pen line 300 700 600 720 8 0.6 0.7
wait 80
expect editor.strokes is 1
key 304 0 1
wait 80
expect editor.strokes is 1
key 305 0 1
wait 80
expect editor.strokes is 0
pentap 1768 1114
wait 120
expect editor.page is 2
key 546 0 1
wait 80
expect editor.page is 2
key 316 0 1
wait 80
expect editor.page is 1
# Reset: shortcuts.ini's buttons again
pentap 1768 1566
wait 300
tap 1535 95
wait 200
tap 945 1930
wait 200
tap 930 1215
wait 200
tap 930 1535
wait 100
expect pads.undo is South
expect pads.previous-page is Select,Left
quit
