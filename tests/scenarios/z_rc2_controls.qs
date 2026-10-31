# settings: status_size=0
# rust-only
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 300
expect settings.status_on is 1
expect settings.status_mode is 1
expect settings.status_size is 0
expect settings.status_corner is 0
expect status.reserved is 0
tap panel shapes
expect editor.shapes_menu is 1
expect settings.shapes is 1
expect settings.shape_quads is 1
expect settings.shape_round is 1
expect settings.shape_links is 1
tap panel shapes.quads
expect settings.shape_quads is 2
tap panel shapes.quads
expect settings.shape_quads is 0
tap panel shapes.quads
expect settings.shape_quads is 1
tap panel shapes.round
expect settings.shape_round is 2
tap panel shapes.round
expect settings.shape_round is 0
tap panel shapes.round
expect settings.shape_round is 1
tap panel shapes.links
expect settings.shape_links is 0
tap panel shapes.links
expect settings.shape_links is 1
tap panel shapes.hold
expect settings.shape_hold_ms is 900
tap panel shapes.hold
expect settings.shape_hold_ms is 400
tap panel shapes.hold
expect settings.shape_hold_ms is 550
tap panel shapes.on
expect settings.shapes is 0
tap panel shapes.on
expect settings.shapes is 1
shot 01-shapes-menu
tap panel shapes.done
expect editor.shapes_menu is 0
tap panel tool-picker
expect editor.shapes_menu is 0
tap panel picker.pin-mode
tap panel picker.tool:highlighter
tap panel picker.tool:fineliner
tap panel picker.tool:marker
expect editor.pins is highlighter,fineliner,marker
tap panel picker.tool:calligraphy
expect editor.pins is highlighter,fineliner,marker
expect toast is The toolbar holds up to 3 pens. Unpin one first.
shot 02-three-pins-tool-picker
tap panel picker.pin-mode
tap panel picker.tool:pencil
expect editor.panel_fits is 1
tap panel links
expect editor.bar is 0
expect editor.link_mode is 1
tap panel links
expect editor.link_mode is 0
rotate 1
wait 200
expect editor.panel_button is 93
tap panel shapes
expect editor.shapes_menu is 1
shot 03-shapes-menu-landscape
tap panel shapes.done
# The added Settings cog opens main Settings and Done restores this notebook.
tap panel settings
expect screen is editor
expect editor.modal is settings
tap library done
expect screen is editor
expect editor.modal is none
expect editor.pins is highlighter,fineliner,marker
expect editor.panel_button is 93
tap panel display
expect editor.modal is display
shot 04-display-without-status-mode
quit
