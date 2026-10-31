# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_SIM_MAP_NOMEM=1
wait 300
tap 200 400
tap panel pages
tap panel pages.graph
expect editor.map is 0
expect editor.modal is pages
expect editor.pages_tab is pages
expect toast is Not enough memory for the graph.
