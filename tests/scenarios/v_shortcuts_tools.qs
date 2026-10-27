# settings: shortcuts_enabled=1
# settings: panel_tools=highlighter,fineliner
# settings: current_tool=3
# settings: before_eraser=7
# settings: draw_tool=5
# Previous and next tool (R1 311, L1 310) go through the tool picker's nine pens in its order,
# left to right and down, and round again. A pinned pen (highlighter, fineliner) lights its pin and
# the tool button keeps its pen, as a pinned pen picked in the picker does. The eraser is not one of
# them: from it they step from the pen it goes back to, its pen from before (a pin or the tool
# button's) or, once that is off the panel, the tool button's. In the open picker they move its
# choice and it stays open; while it pins they do nothing.
wait 150
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect editor.pins is highlighter,fineliner
# The eraser, the marker before it no longer on the panel: next steps from the tool button's fountain pen
expect editor.tool is eraser
expect editor.draw_tool is fountain
key 311 0 1
wait 60
expect editor.tool is calligraphy
expect editor.tool_button is calligraphy
key 311 0 1
wait 60
expect editor.tool is highlighter
expect editor.tool_button is calligraphy
key 311 0 1
wait 60
expect editor.tool is pencil
expect editor.tool_button is pencil
key 311 0 1
wait 60
expect editor.tool is mech
key 311 0 1
wait 60
expect editor.tool is charcoal
key 311 0 1
wait 60
expect editor.tool is ballpoint
key 311 0 1
wait 60
expect editor.tool is fineliner
expect editor.tool_button is ballpoint
key 311 0 1
wait 60
expect editor.tool is marker
key 311 0 1
wait 60
expect editor.tool is fountain
key 310 0 1
wait 60
expect editor.tool is marker
key 310 0 1
wait 60
expect editor.tool is fineliner
key 310 0 1
wait 60
expect editor.tool is ballpoint
key 310 0 1
wait 60
expect editor.tool is charcoal
key 310 0 1
wait 60
expect editor.tool is mech
key 310 0 1
wait 60
expect editor.tool is pencil
key 310 0 1
wait 60
expect editor.tool is highlighter
key 310 0 1
wait 60
expect editor.tool is calligraphy
# Marker (key 6), the eraser (East), next: the fountain pen; marker, eraser, previous: the fineliner
key 7 0
key 305 0 1
wait 60
expect editor.tool is eraser
expect editor.before_eraser is marker
key 311 0 1
wait 60
expect editor.tool is fountain
key 7 0
key 305 0 1
key 310 0 1
wait 60
expect editor.tool is fineliner
# From the pinned fineliner, the eraser, next: the marker
key 305 0 1
key 311 0 1
wait 60
expect editor.tool is marker
# The open picker: next and previous move its choice and leave it open
pentap 1768 358
wait 100
expect editor.modal is tools
key 311 0 1
wait 60
expect editor.modal is tools
expect editor.tool is fountain
key 310 0 1
key 310 0 1
wait 60
expect editor.tool is fineliner
expect editor.modal is tools
# Another shortcut waits for it to close (key 1, the pencil)
key 2 0
wait 60
expect editor.tool is fineliner
# Pin mode: nothing
pentap 1600 273
wait 100
expect editor.pin_mode is 1
key 311 0 1
wait 60
expect editor.tool is fineliner
tap 100 200
wait 100
expect editor.modal is none
quit
