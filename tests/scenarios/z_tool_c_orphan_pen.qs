# settings: status_mode=1
# The pen in use always has its button on the panel. Here settings.ini is as 1.0.0-beta leaves it
# after a pinned highlighter was in use (it drops panel_tools and keeps current_tool), with white
# on: opening a notebook, the highlighter goes to the tool button, selected, with the white mark.
# A pin that stays (the marker's) keeps its pen where it is.
# settings: mirror_pdf=0
# settings: current_tool=4
# settings: draw_tool=0
# settings: white=1
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.pins is
expect editor.tool is highlighter
expect editor.draw_tool is highlighter
expect editor.tool_button is highlighter
expect editor.white_mark is 1
shot 01-the-pen-in-use-on-the-tool-button
# Collapsed and expanded: the same
pentap panel collapse
wait 300
expect editor.panel is collapsed
expect editor.tool_button is
expect editor.white_mark is 1
tap panel expand
wait 300
expect editor.tool_button is highlighter
expect editor.white_mark is 1
# The marker pinned and in use: it stays on its pin through the Library and back
pentap panel tool-picker
wait 200
pentap panel picker.pin-mode
wait 200
pentap panel picker.tool:marker
wait 200
expect editor.pins is marker
pentap panel picker.pin-mode
wait 200
pentap 300 1800
wait 200
pentap panel tool:marker
wait 100
expect editor.tool is marker
expect editor.white_mark is 1
pentap panel library
wait 500
expect screen is library
tap 326 608
wait 500
expect screen is editor
expect editor.tool is marker
expect editor.draw_tool is highlighter
expect editor.pins is marker
expect editor.white_mark is 1
pentap panel library
wait 500
expect screen is library
quit
