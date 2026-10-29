# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_SIM_MAP_NOMEM=1
wait 300
tap 200 400
key 37 0
tap 1590 2370
expect editor.map is 0
expect editor.modal is pages
expect editor.pages_tab is index
expect toast is Not enough memory for the map.
