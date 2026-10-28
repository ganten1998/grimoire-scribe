# settings: status_mode=1
# settings: shortcuts_enabled=1
# Controllers as express keys: one another app holds, one coming and going (a toast each way, at
# most one a direction in 30 s, so a link that keeps dropping does not flash the screen), the
# Controller sheet listing what is connected, and the Buttons sheet learning buttons into
# controller.ini in its groups (one button per action, a toast when a button leaves another action,
# previous / next page, tool and in the tool picker in two presses, 10 s without a press gives up,
# Reset goes back to shortcuts.ini's), which the editor then uses (Add a link and Name the page
# too, learned in the Pages group).
# rust-only
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
tap 746 1930
wait 200
expect library.sheet is shortcuts
shot 01-controller-sheet
tap 930 1215
wait 200
expect library.sheet is buttons
expect library.buttons_group is
shot 02-buttons
tap 930 629
wait 100
expect library.buttons_group is editing
shot 02-buttons-editing
tap 930 576
wait 100
expect library.learning is undo
shot 03-learning
key 305 0 1
wait 100
expect library.learning is
expect pads.undo is East
expect pads.tool:eraser is
expect toast is That button is no longer for Eraser.
tap 930 1696
wait 100
expect library.buttons_group is
tap 930 949
wait 100
expect library.buttons_group is pages
tap 930 469
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
# Add a link and Name the page, under the Index
tap 930 1589
wait 100
expect library.learning is add-link
key 321 0 1
wait 100
tap 930 1749
wait 100
expect library.learning is name-page
key 322 0 1
wait 100
expect library.learning is
expect pads.add-link is 321
expect pads.name-page is 322
shot 04-pages-links
tap 930 1909
wait 100
# Tools: each pen on its own (the picker's order), and previous / next tool
tap 930 789
wait 100
expect library.buttons_group is tools
shot 04-tools
tap 930 1475
wait 100
expect library.learning is tool:calligraphy
key 318 0 1
wait 100
expect pads.tool:calligraphy is 318
tap 930 1795
wait 100
expect library.learning is previous-tool
key 318 0 1
wait 100
expect toast is That button is no longer for Calligraphy pen.
expect library.learning is next-tool
expect pads.tool:calligraphy is
expect pads.previous-tool is 318
tap 930 1795
wait 100
expect library.learning is
tap 930 1955
wait 100
expect library.learning is picker-previous
key 319 0 1
wait 100
expect library.learning is picker-next
key 320 0 1
wait 100
expect library.learning is
expect pads.picker-previous is 319
expect pads.picker-next is 320
expect library.buttons.picker-next is 320
shot 04-tools-picker
tap 930 2115
wait 100
tap 930 629
wait 100
tap 930 736
wait 100
expect library.learning is redo
wait 10300
expect library.learning is
expect pads.redo is North
shot 05-learned
tap 930 1696
wait 100
tap 930 1589
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
pentap panel next
wait 120
expect editor.page is 2
key 546 0 1
wait 80
expect editor.page is 2
key 316 0 1
wait 80
expect editor.page is 1
expect editor.tool is pencil
key 318 0 1
wait 80
expect editor.tool is highlighter
key 320 0 1
wait 80
expect editor.tool is pencil
key 321 0 1
wait 80
expect editor.link_mode is 1
key 321 0 1
wait 80
expect editor.link_mode is 0
key 322 0 1
wait 80
expect editor.keyboard is Page name
tap 217 2365
wait 80
expect editor.modal is none
# Reset: shortcuts.ini's buttons again
pentap panel library
wait 300
tap 1535 95
wait 200
tap 746 1930
wait 200
tap 930 1215
wait 200
tap 930 1429
wait 100
expect pads.undo is South
expect pads.previous-page is Select,Left
expect pads.previous-tool is L1,Up
expect pads.add-link is
quit
