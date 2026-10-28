# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=highlighter,fineliner
# settings: current_tool=3
# settings: before_eraser=7
# settings: draw_tool=5
# copy: tests/fixtures/tool-steps .
# Next and previous tool (R1 311, L1 310; PageDown and PageUp) go through the pens on the panel
# alone: the tool button's (the fountain pen) and the pinned ones (highlighter, fineliner), top to
# bottom and round again, so pressed again and again the highlight never leaves the panel. The
# eraser is not one of them: from it they step from the pen it goes back to, its pen from before (a
# pin or the tool button's) or, once that is off the panel, the tool button's. The tool picker's
# next and previous (R2 313, L2 312, from controller.ini) go through its nine pens in its order: with
# it closed they switch tools (a pinned pen lights its pin and the tool button keeps its pen, as a
# pinned pen picked in the picker does); with it open they move its choice and it stays open, where
# next and previous tool leave it as it is; while it pins they do nothing. With fewer than two pens
# on the panel, next and previous tool go through the picker's nine.
# rust-only
wait 150
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect editor.pins is highlighter,fineliner
expect pads.next-tool is R1
expect pads.picker-next is R2
expect pads.picker-previous is L2
# The eraser, the marker before it no longer on the panel: next steps from the tool button's fountain
# pen, then down the pins and round to the tool button
expect editor.tool is eraser
expect editor.tool_button is fountain
key 311 0 1
wait 60
expect editor.tool is highlighter
expect editor.tool_button is fountain
key 311 0 1
wait 60
expect editor.tool is fineliner
key 311 0 1
wait 60
expect editor.tool is fountain
key 311 0 1
wait 60
expect editor.tool is highlighter
expect editor.tool_button is fountain
expect editor.modal is none
key 310 0 1
wait 60
expect editor.tool is fountain
key 310 0 1
wait 60
expect editor.tool is fineliner
key 310 0 1
wait 60
expect editor.tool is highlighter
key 109 0
wait 60
expect editor.tool is fineliner
key 104 0
wait 60
expect editor.tool is highlighter
# The highlighter's pin, the eraser (East), next: the fineliner; the fineliner, the eraser, previous:
# the highlighter
key 305 0 1
wait 60
expect editor.tool is eraser
expect editor.before_eraser is highlighter
key 311 0 1
wait 60
expect editor.tool is fineliner
key 305 0 1
key 310 0 1
wait 60
expect editor.tool is highlighter
# The picker's next with it closed: its nine, from the highlighter round to the pencil, which goes to
# the tool button; previous back to the highlighter (its pin), the calligraphy pen, the fountain pen
key 313 0 1
wait 60
expect editor.tool is pencil
expect editor.tool_button is pencil
expect editor.modal is none
key 313 0 1
wait 60
expect editor.tool is mech
expect editor.tool_button is mech
key 312 0 1
key 312 0 1
wait 60
expect editor.tool is highlighter
expect editor.tool_button is pencil
key 312 0 1
wait 60
expect editor.tool is calligraphy
expect editor.tool_button is calligraphy
key 312 0 1
wait 60
expect editor.tool is fountain
expect editor.tool_button is fountain
# Next tool goes through the panel's pens as they are now
key 311 0 1
wait 60
expect editor.tool is highlighter
key 311 0 1
key 311 0 1
wait 60
expect editor.tool is fountain
# From the eraser the picker's next steps from the pen it goes back to too (the fountain pen)
key 305 0 1
key 313 0 1
wait 60
expect editor.tool is calligraphy
expect editor.tool_button is calligraphy
# The open picker: next and previous tool leave its choice where it is
pentap panel tool-picker
wait 100
expect editor.modal is tools
key 311 0 1
wait 60
expect editor.modal is tools
expect editor.tool is calligraphy
key 310 0 1
wait 60
expect editor.tool is calligraphy
expect editor.tool_button is calligraphy
# Its own next and previous move its choice and leave it open
key 313 0 1
wait 60
expect editor.modal is tools
expect editor.tool is highlighter
expect editor.tool_button is calligraphy
key 313 0 1
wait 60
expect editor.tool is pencil
expect editor.tool_button is pencil
key 312 0 1
key 312 0 1
wait 60
expect editor.tool is calligraphy
expect editor.modal is tools
shot 01-picker-choice
# Another shortcut waits for it to close (key 1, the pencil)
key 2 0
wait 60
expect editor.tool is calligraphy
# Pin mode: nothing
pentap 1600 273
wait 100
expect editor.pin_mode is 1
key 313 0 1
wait 60
expect editor.tool is calligraphy
# Both pens unpinned there: one pen on the panel, and next and previous tool go through the picker's
# nine, the tool button showing each
pentap 1504 727
wait 200
pentap 1188 561
wait 200
expect editor.pins is
pentap 1600 273
wait 100
tap 100 200
wait 100
expect editor.modal is none
expect editor.tool is calligraphy
key 311 0 1
wait 60
expect editor.tool is highlighter
expect editor.tool_button is highlighter
key 311 0 1
wait 60
expect editor.tool is pencil
key 310 0 1
key 310 0 1
wait 60
expect editor.tool is calligraphy
expect editor.tool_button is calligraphy
shot 02-one-pen
quit
