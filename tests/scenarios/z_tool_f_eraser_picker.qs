# settings: status_mode=1
# The fixed eraser gets the same select-then-open behavior as any other pinned tool.
# settings: draw_tool=2
# settings: current_tool=2
# settings: panel_tools=marker
# settings: mirror_pdf=0
# rust-only
wait 100
tap 965 95
tap 510 460
tap 1564 2254
expect editor.tool is ballpoint
pentap panel eraser
expect editor.tool is eraser
pentap panel tool-picker
expect editor.tool is ballpoint
expect editor.modal is none
pentap panel tool-picker
expect editor.modal is tools
pentap 300 1800
pentap panel tool:marker
expect editor.tool is marker
pentap panel tool-picker
expect editor.tool is ballpoint
expect editor.modal is none
pentap panel tool-picker
expect editor.modal is tools
quit
