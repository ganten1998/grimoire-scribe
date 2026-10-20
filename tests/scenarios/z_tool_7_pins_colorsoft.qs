# Four pinned pens (panel_tools in settings.ini, as the tool picker's Pin mode writes it) on a
# 1986x2648 screen (the Scribe 3 and the Scribe Colorsoft): the panel fits upright and sideways
# with its buttons at full size and its 12 px margins (editor.panel_rect: x y w h), the pins in
# their order under the tool button; collapsed it is the same square as ever. (The 1860x2480
# Scribes, sideways: z_tool_2_pins; every count of pins on both: tests/test_panel.c.)
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1986x2648
# settings: panel_tools=highlighter,fineliner,marker,calligraphy
# settings: mirror_pdf=0
wait 300
expect size is 1986x2648
expect settings.panel_tools is highlighter,fineliner,marker,calligraphy
pentap 347 636
wait 300
expect library.folder is School
tap 347 636
wait 500
expect screen is editor
expect editor.pins is highlighter,fineliner,marker,calligraphy
expect editor.panel_rect is 1830 220 128 1842
expect editor.panel_button is 100
shot 01-upright-four-pins
# The first pin, the highlighter, picked (under the tool button, 108 px apart)
pentap 1894 466
wait 100
expect editor.tool is highlighter
rotate 1
wait 300
expect size is 2648x1986
expect editor.panel_rect is 2492 132 128 1842
expect editor.panel_button is 100
shot 02-sideways-four-pins
# Collapsed sideways: the same square, where the panel's top right was
pentap 2557 172
wait 300
expect editor.panel is collapsed
expect editor.panel_rect is 2500 132 120 120
tap 2560 192
wait 300
expect editor.panel is expanded
expect editor.panel_rect is 2492 132 128 1842
# Upright again: the panel where it was put sideways, as far right as it goes
rotate 0
wait 300
expect editor.panel_rect is 1858 132 128 1842
expect editor.panel_button is 100
pentap 1922 1910
wait 500
expect screen is library
quit
