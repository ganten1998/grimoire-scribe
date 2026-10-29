# settings: status_mode=1
# The panel with 0 to 4 pinned pens on the 2022 and 2024 Scribes' screen (1860x2480), upright and
# sideways: always inside the screen with 12 px free above and below it, its buttons at full size
# (editor.panel_fits), but for four pins sideways, where they are 1% smaller. Each pen is pinned in the
# tool picker's Pin mode. (The Scribe 3's and the Colorsoft's screen: z_tool_a_fit_colorsoft.)
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
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 1
rotate 0
wait 300
# One pin
pentap panel tool-picker
wait 200
pentap 1600 273
wait 200
pentap 1504 727
wait 200
pentap 1600 273
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
pentap 1600 273
wait 200
pentap 1188 561
wait 200
pentap 1600 273
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
pentap 1600 273
wait 200
pentap 1504 561
wait 200
pentap 1600 273
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner,marker
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 0
expect editor.panel_button is 99
rotate 0
wait 300
expect editor.panel_fits is 1
# Four
pentap panel tool-picker
wait 200
pentap 1600 273
wait 200
pentap 1188 727
wait 200
pentap 1600 273
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner,marker,calligraphy
expect editor.panel_fits is 1
rotate 1
wait 300
# (the one place the buttons do not stay at full size: with the Links button there are twelve, and four pins
# sideways need the room between them at half and the buttons 1% smaller: 99 px, the panel 1830 px)
expect editor.panel_fits is 0
expect editor.panel_button is 93
shot 01-four-pins-sideways
rotate 0
wait 300
expect editor.panel_fits is 1
quit
