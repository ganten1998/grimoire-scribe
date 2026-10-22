# The pen the eraser goes back to is kept in settings.ini (before_eraser), so it outlives a restart
# (KOReader's hand-over restarts Grimoire): Grimoire starts with the eraser in use after the pinned
# marker, and a tap on the eraser goes back to the marker, not to the tool button's pencil. While
# the eraser is in use the pen's button (side_button=3) draws white with that same pen.
# settings: mirror_pdf=0
# settings: side_button=3
# settings: panel_tools=marker
# settings: current_tool=3
# settings: before_eraser=7
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.tool is eraser
expect editor.before_eraser is marker
expect editor.draw_tool is pencil
pentap 1768 574
wait 100
expect editor.tool is marker
# A pencil line (the tool button's pencil: with a pin in use, one tap on the tool button picks it)
pentap 1768 358
wait 200
expect editor.tool is pencil
pen line 300 1000 1500 1000 60 0.7 0.7
wait 300
expect editor.strokes is 1
# The marker's pin, then the eraser: the button held draws white marker over the line
pentap 1768 466
wait 100
pentap 1768 574
wait 100
expect editor.tool is eraser
expect editor.before_eraser is marker
side on
pen line 900 900 900 1100 30 0.9 0.9
side off
wait 500
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.stroke_look is white marker
expect editor.stroke_settings is marker
expect editor.live_matches_render is 1
expect editor.tool is eraser
# The pencil (the tool button's) before the eraser: then the button draws white pencil
pentap 1768 574
wait 100
expect editor.tool is marker
pentap 1768 358
wait 200
pentap 1768 574
wait 100
expect editor.before_eraser is pencil
side on
pen line 1200 900 1200 1100 30 0.9 0.9
side off
wait 500
expect editor.strokes is 3
expect editor.stroke_look is white pencil
# The marker unpinned while the eraser is in use: the button draws with the tool button's pencil,
# and a tap on the eraser goes back to it
pentap 1768 466
wait 100
pentap 1768 574
wait 100
expect editor.before_eraser is marker
pentap 1768 358
wait 200
pentap 1600 273
wait 200
pentap 1504 561
wait 200
expect editor.pins is
pentap 1600 273
wait 200
pentap 300 1800
wait 200
expect editor.tool is eraser
side on
pen line 600 900 600 1100 30 0.9 0.9
side off
wait 500
expect editor.strokes is 4
expect editor.stroke_look is white pencil
pentap 1768 466
wait 100
expect editor.tool is pencil
pentap 1768 1566
wait 500
expect screen is library
quit
