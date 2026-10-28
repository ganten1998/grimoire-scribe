# The panel with 0 to 4 pinned pens on the Scribe 3's and the Scribe Colorsoft's screen
# (1986x2648), upright and sideways: always inside the screen with 12 px free above and below it,
# its buttons at full size (editor.panel_fits), as on the 1860x2480 Scribes (z_tool_9_fit). Each
# pen is pinned in the tool picker's Pin mode, in a notebook b_library made.
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1986x2648
# settings: mirror_pdf=0
# rust-only
wait 300
expect size is 1986x2648
pentap 347 636
wait 300
expect library.folder is School
tap 347 636
wait 500
expect screen is editor
expect editor.pins is
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 1
rotate 0
wait 300
# One pin
pentap panel tool-picker
wait 200
pentap 1726 273
wait 200
pentap 1630 727
wait 200
pentap 1726 273
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 1
rotate 0
wait 300
expect editor.panel_fits is 1
# Two
pentap panel tool-picker
wait 200
pentap 1726 273
wait 200
pentap 1314 561
wait 200
pentap 1726 273
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 1
rotate 0
wait 300
expect editor.panel_fits is 1
# Three
pentap panel tool-picker
wait 200
pentap 1726 273
wait 200
pentap 1630 561
wait 200
pentap 1726 273
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner,marker
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 1
rotate 0
wait 300
expect editor.panel_fits is 1
# Four
pentap panel tool-picker
wait 200
pentap 1726 273
wait 200
pentap 1314 727
wait 200
pentap 1726 273
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner,marker,calligraphy
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 1
shot 01-four-pins-sideways
rotate 0
wait 300
expect editor.panel_fits is 1
quit
