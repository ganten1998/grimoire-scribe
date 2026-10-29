# settings: status_mode=1
# The tool picker stays whole on the screen. Beside a panel dragged to the middle (x 900) neither
# side has room for it, so it goes over the panel, inside the screen's right edge (editor.tools_rect),
# and its Pin and White at that edge work. The picker takes the taps over the panel, and stays
# drawn over it when White repaints the panel. In Pin mode the ink row takes no tap.
# settings: mirror_pdf=0
# settings: panel_x=900
# settings: panel_y=220
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.panel_rect is 900 220 128 1626
pentap panel tool-picker
wait 200
expect editor.modal is tools
expect editor.tools_rect is 734 220 1116 768
expect editor.shapes_menu is 0
expect settings.shapes is 1
expect settings.shape_hold_ms is 550
# Pin, at the picker's top right: Pin mode; White there does nothing; Done
pentap editor picker.pin-mode
wait 200
expect editor.pin_mode is 1
pentap 1591 878
wait 200
expect settings.white is 0
expect editor.pin_mode is 1
expect editor.modal is tools
pentap editor picker.pin-mode
wait 200
expect editor.pin_mode is 0
# White: the picker stays, over the panel, which shows the white mark under it
pentap 1591 878
wait 200
expect settings.white is 1
expect editor.modal is tools
expect editor.white_mark is 1
shot 01-picker-over-the-panel
# The ballpoint, where the picker covers the panel's Undo: picked (the picker closes)
pentap editor picker.tool:ballpoint
wait 200
expect editor.modal is none
expect editor.tool is ballpoint
expect editor.draw_tool is ballpoint
expect editor.strokes is 0
expect settings.shapes is 1
expect settings.shape_hold_ms is 550
shot 02-closed
# Smart Shapes is a separate sheet reached from its own panel button.
pentap panel shapes
expect editor.shapes_menu is 1
pentap panel shapes.done
expect editor.shapes_menu is 0
expect editor.modal is none
expect editor.tool is ballpoint
# Sideways there is room right of the panel
rotate 1
wait 300
pentap panel tool-picker
wait 200
expect editor.modal is tools
expect editor.tools_rect is 826 220 1116 768
pentap 300 1700
wait 200
expect editor.modal is none
rotate 0
wait 300
pentap panel library
wait 500
expect screen is library
quit
