# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_CHECK_FIT=1
# settings: status_on=1
wait 300
tap 200 400
wait 200
tap panel pages
tap panel pages.graph
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
tap panel pages
tap panel pages.graph
expect editor.map is graph
# Around current page, then back to All.
tap panel map.view
expect editor.map.mode is around
shot 02-around
tap panel map.view
expect editor.map.mode is all
expect editor.map.view is all
# A panned view: View first fits All again (its label says Fit), then goes on to Around.
drag pen 900 900 1300 1200 20
expect editor.map.view is fit all
shot 02b-panned
tap panel map.view
expect editor.map.view is all
expect editor.map.mode is all
tap panel map.view
expect editor.map.mode is around
tap panel map.view
expect editor.map.mode is all
rotate 1
wait 300
expect editor.map is graph
shot 03-landscape
rotate 0
wait 200
# List returns to the Index tab; its Graph button returns to the map.
tap panel map.list
wait 200
expect editor.modal is map
expect editor.map.list is 1
shot 04-list
tap panel map.graph
wait 200
expect editor.map is graph
# The map owns Back even with the notebook clock on.
tap 100 95
expect editor.map is 0
expect editor.modal is none
expect settings.status_on is 1
