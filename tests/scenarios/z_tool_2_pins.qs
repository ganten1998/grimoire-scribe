# settings: status_mode=1
# Pinned pens. The tool picker's Pin (top right) turns its grid into the pens pinned to the panel:
# a tap pins one under the tool button or unpins it, the picker staying open; Done ends that. Each
# pin is a panel button, selected while in use, and the buttons below move down by one each. A tap
# on a pin picks it and leaves the tool button's pen; so does a pinned pen picked in the picker.
# While a pin is in use, the tool button's first tap picks its own pen back, and the next opens the
# picker. The eraser, tapped on and off, goes back to the pen used just before it (a pin), or to
# the tool button's pen once that pin is gone. Collapsed and expanded; turned sideways, where three
# pins fit with the panel's margins (the buttons 7% smaller); a fourth pin is refused with a toast; the
# tool button's own pen pinned, the tool button shows a grid, which always opens the picker;
# unpinning the pen in use moves it to the tool button. Last, every pin taken off: the panel is as
# it was.
# settings: mirror_pdf=0
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.pins is
expect settings.panel_tools is
expect editor.panel_rect is 1704 220 128 1734
expect editor.panel_button is 100
expect editor.tool_button is pencil
# Pin mode: Highlighter, then Fineliner
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
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
pentap editor picker.tool:highlighter
wait 200
expect editor.modal is tools
expect editor.pins is highlighter
pentap editor picker.tool:fineliner
wait 200
expect editor.pins is highlighter,fineliner
expect settings.panel_tools is highlighter,fineliner
expect editor.panel_rect is 1704 220 128 1950
expect editor.tool is pencil
shot 02-two-pinned
# Done, then a tap outside closes the picker
pentap editor picker.pin-mode
wait 200
expect editor.pin_mode is 0
expect editor.modal is tools
pentap 300 1800
wait 200
expect editor.modal is none
shot 03-panel-with-pins
# The highlighter's pin (the first under the tool button): it writes, the tool button keeps the pencil
pentap panel tool:highlighter
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
pentap panel tool-picker
wait 200
expect editor.tool is pencil
expect editor.modal is none
pentap panel tool-picker
wait 200
pentap 1425 878
wait 200
expect settings.white is 1
pentap 300 1800
wait 200
expect editor.modal is none
pentap panel tool:highlighter
wait 100
expect editor.tool is highlighter
expect editor.white_mark is 1
shot 03b-white-on-the-pin
pentap panel tool-picker
wait 200
pentap panel tool-picker
wait 200
pentap 951 878
wait 200
expect settings.white is 0
expect editor.white_mark is 0
pentap 300 1800
wait 200
expect editor.modal is none
pentap panel tool:highlighter
wait 100
expect editor.tool is highlighter
# The eraser, now under both pins, on then off: back to the highlighter
pentap panel eraser
wait 100
expect editor.tool is eraser
expect editor.before_eraser is highlighter
pentap panel eraser
wait 100
expect editor.tool is highlighter
# The fineliner's pin, the eraser and back: the fineliner
pentap panel tool:fineliner
wait 100
expect editor.tool is fineliner
pentap panel eraser
wait 100
pentap panel eraser
wait 100
expect editor.tool is fineliner
pen line 300 900 1300 900 60 0.7 0.7
wait 300
expect editor.strokes is 2
expect editor.stroke_look is fineliner
# Undo below the pins, found by name
pentap panel undo
wait 300
expect editor.strokes is 1
# Collapsed: the same square as ever; expanded with its pins
pentap panel collapse
wait 300
expect editor.panel is collapsed
expect editor.panel_rect is 1712 220 120 120
tap panel expand
wait 300
expect editor.panel is expanded
expect editor.panel_rect is 1704 220 128 1950
# Sideways: two pins fit where the panel was (its place kept from the collapse)
rotate 1
wait 300
expect size is 2480x1860
expect editor.panel_rect is 1962 30 128 1830
expect editor.panel_button is 99
shot 04-sideways-two-pins
# The third pin added sideways (Marker): the panel keeps its 12 px margins,
# its buttons 7% smaller; the picker stays where it opened (the fineliner's pin is in use: the
# tool button's first tap picks its pencil)
pentap panel tool-picker
wait 200
expect editor.tool is pencil
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
wait 200
expect editor.pin_mode is 1
pentap editor picker.tool:marker
wait 200
expect editor.pins is highlighter,fineliner,marker
expect settings.panel_tools is highlighter,fineliner,marker
expect editor.panel_rect is 1962 30 128 1820
expect editor.panel_button is 93
# A fourth: refused, with a toast
pentap editor picker.tool:calligraphy
wait 200
expect toast is The panel holds up to 3 pens. Unpin one first.
expect editor.pins is highlighter,fineliner,marker
expect editor.modal is tools
shot 05-sideways-three-pinned
pentap editor picker.pin-mode
wait 200
pentap 300 1500
wait 200
expect editor.modal is none
shot 06-sideways-three-pins
# The third pin writes; the eraser under it, on and off, goes back to it
pentap panel tool:marker
wait 100
expect editor.tool is marker
pen line 300 900 1300 950 40 0.7 0.7
wait 300
expect editor.strokes is 2
pentap panel eraser
wait 100
expect editor.tool is eraser
pentap panel eraser
wait 100
expect editor.tool is marker
rotate 0
wait 300
expect size is 1860x2480
expect editor.panel_rect is 1704 220 128 2058
expect editor.panel_button is 100
shot 07-upright-three-pins
# The tool button's own pen pinned (Fineliner out, Pencil in): the tool button shows a grid (the
# marker pen's pin is in use: the first tap picks the pencil)
pentap panel tool-picker
wait 200
expect editor.tool is pencil
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
wait 200
pentap editor picker.tool:fineliner
wait 200
pentap editor picker.tool:pencil
wait 200
expect editor.pins is highlighter,marker,pencil
expect editor.tool_button is grid
pentap editor picker.pin-mode
wait 200
pentap 300 1800
wait 200
expect editor.modal is none
shot 08-grid-tool-button
# With the highlighter's pin in use, the grid opens the picker (its pencil is a pin of its own);
# closed again. The pencil's pin (the third) picks it; the highlighter picked in the picker (the
# grid opens it) is the one in use, and the tool button keeps its pen
pentap panel tool:highlighter
wait 100
expect editor.tool is highlighter
pentap panel tool-picker
wait 200
expect editor.modal is tools
expect editor.tool is highlighter
pentap 300 1800
wait 200
expect editor.modal is none
pentap panel tool:pencil
wait 100
expect editor.tool is pencil
pentap panel tool-picker
wait 200
pentap editor picker.tool:highlighter
wait 200
expect editor.modal is none
expect editor.tool is highlighter
expect editor.draw_tool is pencil
expect editor.tool_button is grid
# The highlighter, in use, unpinned: it moves to the tool button
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
wait 200
pentap editor picker.tool:highlighter
wait 200
expect editor.pins is marker,pencil
expect editor.tool is highlighter
expect editor.draw_tool is highlighter
expect editor.tool_button is highlighter
pentap editor picker.pin-mode
wait 200
pentap 300 1800
wait 200
# The marker's pin, the eraser, then two top taps select the highlighter and open its picker.
# Unpinning the marker leaves the highlighter selected.
pentap panel tool:marker
wait 100
expect editor.tool is marker
pentap panel eraser
wait 100
expect editor.tool is eraser
expect editor.before_eraser is marker
pentap panel tool-picker
wait 200
expect editor.tool is highlighter
expect editor.modal is none
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
wait 200
pentap editor picker.tool:marker
wait 200
expect editor.pins is pencil
pentap editor picker.pin-mode
wait 200
pentap 300 1800
wait 200
expect editor.tool is highlighter
pentap panel eraser
expect editor.tool is eraser
pentap panel eraser
wait 100
expect editor.tool is highlighter
# Every pin off: the panel as it was
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
wait 200
pentap editor picker.tool:pencil
wait 200
expect editor.pins is
expect settings.panel_tools is
expect editor.panel_rect is 1704 220 128 1734
pentap editor picker.pin-mode
wait 200
pentap 300 1800
wait 200
expect editor.modal is none
shot 09-no-pins
# The fourth-pin refusal never removes Calligraphy from the picker: it still draws normally.
pentap panel tool-picker
pentap panel picker.tool:calligraphy
expect editor.tool is calligraphy
pen line 300 1100 1300 1150 40 0.7 0.7
wait 700
expect editor.strokes is 3
expect editor.stroke_look is calligraphy
expect editor.live_matches_render is 1
pentap panel eraser
expect editor.tool is eraser
pentap panel eraser
expect editor.tool is calligraphy
pentap panel undo
expect editor.strokes is 2
pentap panel library
wait 500
expect screen is library
quit
