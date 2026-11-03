# rust-only
# generate: tests/map_fixture.py 40
# env: GRIMOIRE_CHECK_FIT=1
# The Graph's keys on a keyboard paired with the Kindle: the arrows pan it (Left turns no page there; nothing on a
# page changes), V switches View: All and View: Adjacent (the view button's words: a tap always switches, after a
# pan too), Enter opens the one page
# selected. The fixture's shortcuts.ini (shortcuts enabled) binds none of the Graph's actions: its arrows and V
# come in memory (shortcuts.rs shortcuts_lookup_graph).
wait 300
tap 326 608
expect editor.page is 3
tap panel pages
tap panel pages.graph
expect editor.map is graph
expect editor.map.view is all
key 105 0
expect editor.map is graph
expect editor.map.touched is 1
expect editor.page is 3
key 108 0
key 103 0
key 106 0
expect editor.map is graph
# The view button after a pan: View: Adjacent at once, fitted (no "Fit:" first)
tap panel map.view
expect editor.map.view is adjacent
expect editor.map.touched is 0
shot 01-adjacent
key 47 0
expect editor.map.view is all
# A finger's tap selects; Enter opens it
tap map-node 7
expect editor.map.selected is 7
key 28 0
expect editor.map is 0
expect editor.page is 7
quit
