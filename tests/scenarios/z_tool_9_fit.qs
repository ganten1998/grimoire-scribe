# settings: status_mode=1
# The panel with 0 to 3 pinned pens on the 2022 and 2024 Scribes' screen (1860x2480), upright and
# sideways: always inside the screen with 12 px free above and below it, its buttons at full size
# (editor.panel_fits), but for two pins sideways (99 px) and three pins (93 px). Each pen is pinned in the
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
pentap panel picker.pin-mode
wait 200
pentap panel picker.tool:highlighter
wait 200
pentap panel picker.pin-mode
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
pentap panel picker.pin-mode
wait 200
pentap panel picker.tool:fineliner
wait 200
pentap panel picker.pin-mode
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 0
expect editor.panel_button is 99
rotate 0
wait 300
expect editor.panel_fits is 1
# Three
pentap panel tool-picker
wait 200
pentap panel picker.pin-mode
wait 200
pentap panel picker.tool:marker
wait 200
pentap panel picker.pin-mode
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner,marker
expect editor.panel_fits is 1
rotate 1
wait 300
expect editor.panel_fits is 0
expect editor.panel_button is 93
rotate 0
wait 300
expect editor.panel_fits is 1
# A fourth pin is refused; the existing three and their order stay intact.
pentap panel tool-picker
wait 200
pentap panel picker.pin-mode
wait 200
pentap panel picker.tool:calligraphy
wait 200
expect toast is The panel holds up to 3 pens. Unpin one first.
expect editor.pins is highlighter,fineliner,marker
expect settings.panel_tools is highlighter,fineliner,marker
expect editor.modal is tools
pentap panel picker.pin-mode
wait 200
pentap 300 1800
wait 200
expect editor.pins is highlighter,fineliner,marker
expect editor.panel_fits is 1
rotate 1
wait 300
# Fourteen base buttons plus three pins fit within the margins; buttons are 93 px sideways.
expect editor.panel_fits is 0
expect editor.panel_button is 93
shot 01-three-pins-sideways
rotate 2
wait 300
expect size is 1860x2480
expect editor.panel_fits is 1
expect editor.panel_button is 100
rotate 3
wait 300
expect size is 2480x1860
expect editor.panel_fits is 0
expect editor.panel_button is 93
rotate 0
wait 300
expect editor.panel_fits is 1
quit
