# A remembered pen that is no longer pinned falls back to the top tool after a restart.
# settings: mirror_pdf=0
# settings: side_button=3
# settings: panel_tools=
# settings: current_tool=3
# settings: before_eraser=7
wait 100
tap 965 95
tap 510 460
tap 1564 2254
expect editor.tool is eraser
expect editor.before_eraser is marker
pentap 1768 466
expect editor.tool is pencil
quit
