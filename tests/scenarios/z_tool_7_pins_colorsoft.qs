# settings: status_mode=1
# Three pinned pens (panel_tools in settings.ini, as the tool picker's Pin mode writes it) on a
# 1986x2648 screen (the Scribe 3 and the Scribe Colorsoft): the panel fits upright and sideways
# with its buttons at full size and its 12 px margins (editor.panel_rect: x y w h), the pins in
# their order under the tool button; collapsed it is the same square as ever. (The 1860x2480
# Scribes, sideways: z_tool_2_pins; every count of pins on both: rust/tests/test_panel.rs.)
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1986x2648
# settings: panel_tools=highlighter,fineliner,marker
# settings: mirror_pdf=0
# rust-only
wait 300
expect size is 1986x2648
expect settings.panel_tools is highlighter,fineliner,marker
pentap 347 636
wait 300
expect library.folder is School
tap 347 636
wait 500
expect screen is editor
expect editor.pins is highlighter,fineliner,marker
expect editor.panel_rect is 1830 220 128 1950
expect editor.panel_button is 100
shot 01-upright-three-pins
# The first pin, the highlighter, picked (under the tool button, 108 px apart)
pentap panel tool:highlighter
wait 100
expect editor.tool is highlighter
rotate 1
wait 300
expect size is 2648x1986
expect editor.panel_rect is 2093 24 128 1950
expect editor.panel_button is 100
shot 02-sideways-three-pins
# Collapsed sideways: the same square, where the panel's top right was
pentap panel collapse
wait 300
expect editor.panel is collapsed
expect editor.panel_rect is 2101 24 120 120
tap panel expand
wait 300
expect editor.panel is expanded
expect editor.panel_rect is 2093 24 128 1950
# Upright again: the panel where it was put sideways, as far right as it goes
rotate 0
wait 300
expect editor.panel_rect is 1858 24 128 1950
expect editor.panel_button is 100
pentap panel library
wait 500
expect screen is library
quit
