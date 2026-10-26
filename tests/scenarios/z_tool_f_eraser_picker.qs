# The fixed eraser gets the same select-then-open behavior as any other pinned tool.
# settings: draw_tool=2
# settings: current_tool=2
# settings: panel_tools=marker
# settings: mirror_pdf=0
wait 100
tap 965 95
tap 510 460
tap 1564 2254
expect editor.tool is ballpoint
pentap 1768 574
expect editor.tool is eraser
pentap 1768 358
expect editor.tool is ballpoint
expect editor.modal is none
pentap 1768 358
expect editor.modal is tools
pentap 300 1800
pentap 1768 466
expect editor.tool is marker
pentap 1768 358
expect editor.tool is ballpoint
expect editor.modal is none
pentap 1768 358
expect editor.modal is tools
quit
