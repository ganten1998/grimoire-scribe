# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# The tool picker's ink row shows the ink picked at once (2.0.0-rc.12: it showed the old one until the
# picker was opened again), and so does an open picker when a shortcut turns white ink on or off.
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
tap panel tool-picker
wait 100
expect editor.modal is tools
expect editor.tools.ink is black
expect settings.white is 0
tap panel picker.ink.1
wait 100
expect settings.white is 1
expect editor.tools.ink is white
shot 01-white-picked
tap panel picker.ink.0
wait 100
expect settings.white is 0
expect editor.tools.ink is black
quit
