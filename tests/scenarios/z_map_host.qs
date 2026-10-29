# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_CHECK_FIT=1
# settings: status_on=1
wait 300
tap 200 400
wait 200
key 37 0
expect editor.bar is open
# Links bar bottom Index button.
tap 1590 2370
wait 200
expect editor.map is graph
expect editor.modal is map
expect editor.map.nodes is 5
expect editor.map.layout is done
expect editor.strokes is 0
shot 01-all
# Resolve coordinates from the current host's layout.
tap map-node 3
expect editor.map.selected is 3
shot 01b-selected
tap map-node 3
expect editor.map is 0
expect editor.page is 3
key 48 0
expect editor.page is 1
key 37 0
tap 1590 2370
expect editor.map is graph
# Around current page, then zoom and return to All.
tap 1190 95
expect editor.map.mode is around
shot 02-around
tap 890 95
expect editor.map.mode is all
rotate 1
wait 300
expect editor.map is graph
shot 03-landscape
rotate 0
wait 200
# List returns to the Index tab; its Graph button returns to the map.
tap 730 95
wait 200
expect editor.modal is pages
expect editor.pages_tab is index
shot 04-list
tap 1660 95
wait 200
expect editor.map is graph
# The map owns Back even with the notebook clock on.
tap 100 95
expect editor.map is 0
expect editor.modal is none
expect settings.status_on is 1
