# Pinned pens. The tool picker's Pin (top right) turns its grid into the pens pinned to the panel:
# a tap pins one under the tool button or unpins it, the picker staying open; Done ends that. Each
# pin is a panel button, selected while in use, and the buttons below move down by one each. A tap
# on a pin picks it and leaves the tool button's pen; so does a pinned pen picked in the picker.
# While a pin is in use, the tool button's first tap picks its own pen back, and the next opens the
# picker. The eraser, tapped on and off, goes back to the pen used just before it (a pin), or to
# the tool button's pen once that pin is gone. Collapsed and expanded; turned sideways, where four
# pins fit with the panel's margins and full-size buttons; a fifth pin is refused with a toast; the
# tool button's own pen pinned, the tool button shows a grid, which always opens the picker;
# unpinning the pen in use moves it to the tool button. Last, every pin taken off: the panel is as
# it was.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.pins is
expect settings.panel_tools is
expect editor.panel_rect is 1704 220 128 1410
expect editor.panel_button is 100
expect editor.tool_button is pencil
# Pin mode: Highlighter, then Fineliner
pentap 1768 358
wait 200
pentap 1600 273
wait 200
expect editor.modal is tools
expect editor.pin_mode is 1
shot 01-pin-mode
# The ink row is out of use in Pin mode: White there does nothing
pentap 1425 878
wait 200
expect settings.white is 0
expect editor.pin_mode is 1
expect editor.white_mark is 0
pentap 1504 727
wait 200
expect editor.modal is tools
expect editor.pins is highlighter
pentap 1188 561
wait 200
expect editor.pins is highlighter,fineliner
expect settings.panel_tools is highlighter,fineliner
expect editor.panel_rect is 1704 220 128 1626
expect editor.tool is pencil
shot 02-two-pinned
# Done, then a tap outside closes the picker
pentap 1600 273
wait 200
expect editor.pin_mode is 0
expect editor.modal is tools
pentap 300 1800
wait 200
expect editor.modal is none
shot 03-panel-with-pins
# The highlighter's pin (the first under the tool button): it writes, the tool button keeps the pencil
pentap 1768 466
wait 100
expect editor.tool is highlighter
expect editor.draw_tool is pencil
pen line 300 700 1300 700 60 0.7 0.7
wait 300
expect editor.strokes is 1
expect editor.stroke_look is highlighter
# With a pin in use, a tap on the tool button picks its pencil, and the next opens the picker.
# White there, then the highlighter's pin: the pin shows the white mark (the tool button's pencil
# is not in use); Black again, and the highlighter's pin
pentap 1768 358
wait 200
expect editor.tool is pencil
expect editor.modal is none
pentap 1768 358
wait 200
pentap 1425 878
wait 200
expect settings.white is 1
pentap 300 1800
wait 200
expect editor.modal is none
pentap 1768 466
wait 100
expect editor.tool is highlighter
expect editor.white_mark is 1
shot 03b-white-on-the-pin
pentap 1768 358
wait 200
pentap 1768 358
wait 200
pentap 951 878
wait 200
expect settings.white is 0
expect editor.white_mark is 0
pentap 300 1800
wait 200
expect editor.modal is none
pentap 1768 466
wait 100
expect editor.tool is highlighter
# The eraser, now under both pins, on then off: back to the highlighter
pentap 1768 682
wait 100
expect editor.tool is eraser
expect editor.before_eraser is highlighter
pentap 1768 682
wait 100
expect editor.tool is highlighter
# The fineliner's pin, the eraser and back: the fineliner
pentap 1768 574
wait 100
expect editor.tool is fineliner
pentap 1768 682
wait 100
pentap 1768 682
wait 100
expect editor.tool is fineliner
pen line 300 900 1300 900 60 0.7 0.7
wait 300
expect editor.strokes is 2
expect editor.stroke_look is fineliner
# Undo below the pins (594 + 2 x 108)
pentap 1768 810
wait 300
expect editor.strokes is 1
# Collapsed: the same square as ever; expanded with its pins
pentap 1795 260
wait 300
expect editor.panel is collapsed
expect editor.panel_rect is 1712 220 120 120
tap 1772 280
wait 300
expect editor.panel is expanded
expect editor.panel_rect is 1704 220 128 1626
# Sideways: two pins fit where the panel was (its place kept from the collapse)
rotate 1
wait 300
expect size is 2480x1860
expect editor.panel_rect is 1704 220 128 1626
expect editor.panel_button is 100
shot 04-sideways-two-pins
# Two more pinned sideways (Marker, Calligraphy): the panel moves up to keep its 12 px margins,
# its buttons at full size; the picker stays where it opened (the fineliner's pin is in use: the
# tool button's first tap picks its pencil)
pentap 1768 358
wait 200
expect editor.tool is pencil
pentap 1768 358
wait 200
pentap 1600 273
wait 200
expect editor.pin_mode is 1
pentap 1504 561
wait 200
expect editor.pins is highlighter,fineliner,marker
pentap 1188 727
wait 200
expect editor.pins is highlighter,fineliner,marker,calligraphy
expect editor.panel_rect is 1704 29 128 1819
expect editor.panel_button is 100
# A fifth: refused, with a toast
pentap 872 395
wait 200
expect toast is The panel holds up to 4 pens. Unpin one first.
expect editor.pins is highlighter,fineliner,marker,calligraphy
expect editor.modal is tools
shot 05-sideways-four-pinned
pentap 1600 273
wait 200
pentap 300 1500
wait 200
expect editor.modal is none
shot 06-sideways-four-pins
# The fourth pin writes; the eraser under it, on and off, goes back to it
pentap 1768 594
wait 100
expect editor.tool is calligraphy
pen line 300 900 1300 950 40 0.7 0.7
wait 300
expect editor.strokes is 2
pentap 1768 701
wait 100
expect editor.tool is eraser
pentap 1768 701
wait 100
expect editor.tool is calligraphy
rotate 0
wait 300
expect size is 1860x2480
expect editor.panel_rect is 1704 220 128 1842
expect editor.panel_button is 100
shot 07-upright-four-pins
# The tool button's own pen pinned (Fineliner out, Pencil in): the tool button shows a grid (the
# calligraphy pen's pin is in use: the first tap picks the pencil)
pentap 1768 358
wait 200
expect editor.tool is pencil
pentap 1768 358
wait 200
pentap 1600 273
wait 200
pentap 1188 561
wait 200
pentap 872 395
wait 200
expect editor.pins is highlighter,marker,calligraphy,pencil
expect editor.tool_button is grid
pentap 1600 273
wait 200
pentap 300 1800
wait 200
expect editor.modal is none
shot 08-grid-tool-button
# With the highlighter's pin in use, the grid opens the picker (its pencil is a pin of its own);
# closed again. The pencil's pin (the fourth) picks it; the highlighter picked in the picker (the
# grid opens it) is the one in use, and the tool button keeps its pen
pentap 1768 466
wait 100
expect editor.tool is highlighter
pentap 1768 358
wait 200
expect editor.modal is tools
expect editor.tool is highlighter
pentap 300 1800
wait 200
expect editor.modal is none
pentap 1768 790
wait 100
expect editor.tool is pencil
pentap 1768 358
wait 200
pentap 1504 727
wait 200
expect editor.modal is none
expect editor.tool is highlighter
expect editor.draw_tool is pencil
expect editor.tool_button is grid
# The highlighter, in use, unpinned: it moves to the tool button
pentap 1768 358
wait 200
pentap 1600 273
wait 200
pentap 1504 727
wait 200
expect editor.pins is marker,calligraphy,pencil
expect editor.tool is highlighter
expect editor.draw_tool is highlighter
expect editor.tool_button is highlighter
pentap 1600 273
wait 200
pentap 300 1800
wait 200
# The marker's pin, the eraser, then the marker unpinned while the eraser is in use: the eraser
# goes back to the tool button's pen
pentap 1768 466
wait 100
expect editor.tool is marker
pentap 1768 790
wait 100
expect editor.tool is eraser
expect editor.before_eraser is marker
pentap 1768 358
wait 200
pentap 1600 273
wait 200
pentap 1504 561
wait 200
expect editor.pins is calligraphy,pencil
pentap 1600 273
wait 200
pentap 300 1800
wait 200
pentap 1768 682
wait 100
expect editor.tool is highlighter
# Every pin off: the panel as it was
pentap 1768 358
wait 200
pentap 1600 273
wait 200
pentap 1188 727
wait 200
pentap 872 395
wait 200
expect editor.pins is
expect settings.panel_tools is
expect editor.panel_rect is 1704 220 128 1410
pentap 1600 273
wait 200
pentap 300 1800
wait 200
expect editor.modal is none
shot 09-no-pins
pentap 1768 1566
wait 500
expect screen is library
quit
