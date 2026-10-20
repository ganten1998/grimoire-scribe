# White ink from the tool picker's ink row (Black | White): a tap keeps the picker open and sets
# settings.white; the pen in use then shows that it draws white (its button hollow: editor.white_mark),
# on the collapsed panel too, and draws white ink, an erase stroke with the pen's own look. The
# eraser in use shows no mark. Black again: the pen draws ink, and the mark goes.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect settings.white is 0
expect editor.white_mark is 0
pen line 300 700 1300 700 60 0.7 0.7
wait 300
expect editor.strokes is 1
pentap 1768 358
wait 200
expect editor.modal is tools
expect editor.pin_mode is 0
shot 01-picker-black
# White: the picker stays, the pencil's button turns hollow behind it
pentap 1425 878
wait 200
expect editor.modal is tools
expect settings.white is 1
expect editor.white_mark is 1
expect editor.tool is pencil
shot 02-picker-white
# A tap outside closes it (and draws nothing)
pentap 300 1500
wait 200
expect editor.modal is none
expect editor.strokes is 1
# White pencil through the line: an erase stroke, kept like ink
pen line 700 640 760 760 30 0.9 0.9
wait 700
shot 03-white-through-the-line
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.stroke_look is white pencil
expect editor.live_matches_render is 1
expect editor.white_mark is 1
# Collapsed, the square shows it too; expanded again
pentap 1795 260
wait 300
expect editor.panel is collapsed
expect editor.white_mark is 1
shot 04-collapsed-white
tap 1772 280
wait 300
expect editor.panel is expanded
expect editor.white_mark is 1
# The eraser in use: no mark; back to the pencil, the mark again
pentap 1768 466
wait 100
expect editor.tool is eraser
expect editor.white_mark is 0
pentap 1768 466
wait 100
expect editor.tool is pencil
expect editor.white_mark is 1
# Another pen picked keeps white: the marker draws white marker
pentap 1768 358
wait 200
pentap 1504 561
wait 200
expect editor.modal is none
expect editor.tool is marker
expect settings.white is 1
expect editor.white_mark is 1
pen line 1000 640 1060 760 30 0.9 0.9
wait 700
expect editor.strokes is 3
expect editor.erase_strokes is 2
expect editor.stroke_look is white marker
# Black again: ink, and no mark
pentap 1768 358
wait 200
pentap 951 878
wait 200
expect editor.modal is tools
expect settings.white is 0
expect editor.white_mark is 0
shot 05-picker-black-again
pentap 300 1500
wait 200
expect editor.modal is none
pen line 300 900 1300 900 60 0.7 0.7
wait 700
expect editor.strokes is 4
expect editor.erase_strokes is 2
expect editor.stroke_look is marker
expect editor.live_matches_render is 1
pentap 1768 1566
wait 500
expect screen is library
quit
