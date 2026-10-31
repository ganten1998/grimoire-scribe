# rust-only
# root: z_bar_0_make
# settings: status_mode=1
# settings: shapes=1
# settings: shape_corner=0
# settings: shape_min_mm=0
# The Smart shapes menu's two sliders (2.0.0-rc.12), upright and sideways, by finger and by pen. The
# menu's place is checked first, then the tracks are pressed by coordinates: upright the menu is at
# 568,220, each track starts 589 px into it and is 459 px long, Corner roundness's row is centred at
# y 934 and Smallest shape's at 1070 (the rows' hits take the gaps beside them too). A press answers at
# once, wherever it lands on the track (no drag from the knob needed); a press on a slider's name moves
# nothing until it comes over the track; a cancelled touch (a palm, the pen coming near) keeps what the
# press or drag had set; a finger outside the menu, cancelled when the pen comes down, closes nothing,
# and the pen drags the slider.
wait 300
tap 200 400
wait 300
pen leave
wait 1100
tap panel shapes
wait 100
expect editor.shapes_menu is 1
expect editor.tools_rect is 568 220 1116 1212
shot 01-menu
# Corner roundness: a press halfway along the track sets 50% at the press, then the drag follows,
# off the row too, and past the end.
touch down 1386 934
expect settings.shape_corner is 50
touch move 1501 934
expect settings.shape_corner is 75
touch move 1500 1000
expect settings.shape_corner is 75
touch move 2000 934
expect settings.shape_corner is 100
touch up 2000 934
expect settings.shape_corner is 100
shot 02-corner-full
# A press on the name moves nothing; slid onto the track it follows (here to its left end: Sharp).
touch down 700 934
expect settings.shape_corner is 100
touch up 700 934
expect settings.shape_corner is 100
touch down 700 934
touch move 1157 934
expect settings.shape_corner is 0
touch up 1157 934
expect settings.shape_corner is 0
expect editor.shapes_menu is 1
# Smallest shape: a tap on the track, then one in the gap below the row.
tap 1386 1070
expect settings.shape_min_mm is 20
tap 1272 1136
expect settings.shape_min_mm is 10
# A drag a palm cancels keeps the value it had come to (the cancel's place is off the screen).
finger 1 down 1501 1070
frame
expect settings.shape_min_mm is 30
finger 1 move 1386 1070
frame
expect settings.shape_min_mm is 20
finger 2 down 1300 2000 0 2
frame
expect settings.shape_min_mm is 20
finger 1 up 1386 1070
finger 2 up 1300 2000
frame
wait 100
expect settings.shape_min_mm is 20
expect editor.shapes_menu is 1
shot 03-smallest-20
# A press cut short by the pen coming near (the Kindle cancels a touch the moment the pen arrives,
# anywhere on the screen, often before the finger has moved a pixel): the press alone has set the
# value, and the cancel keeps it. Before, the knob moved only with a move or at a lift, so the press
# did nothing.
finger 1 down 1616 1070
frame
pen hover 300 2200
expect settings.shape_min_mm is 40
finger 1 up 1616 1070
frame
pen leave
wait 100
expect settings.shape_min_mm is 40
# A finger resting outside the menu, then the pen on Corner roundness: the finger is cancelled as the
# pen lands, the menu stays, and the pen drags the knob.
finger 1 down 300 2000
frame
pen down 1272 934 0.6 0 0
expect settings.shape_corner is 25
pen move 1386 934 0.6 0 0
pen up 1386 934
expect settings.shape_corner is 50
finger 1 up 300 2000
frame
pen leave
wait 100
expect editor.shapes_menu is 1
expect editor.strokes is 0
# Sideways: the menu at 826,126, the rows centred at y 840 and 976, the tracks from x 1415.
rotate 1
wait 300
expect editor.tools_rect is 826 126 1116 1212
shot 04-menu-sideways
touch down 1530 840
expect settings.shape_corner is 25
touch up 1530 840
tap 1553 976
expect settings.shape_min_mm is 12
shot 05-sideways-set
rotate 0
wait 300
tap panel shapes.done
expect editor.shapes_menu is 0
quit
