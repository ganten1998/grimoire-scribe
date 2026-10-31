# rust-only
# generate: tests/map_fixture.py 40
# A pen's tap in the Graph selects a node as a finger's does, and a second tap opens it (2.0.0-rc.12: a pen's
# tap often reached the Kindle as two, which selected and opened at once); a second pen tap within 450 ms of
# the one that selected is the stylus bouncing.
wait 300
tap 326 608
tap panel pages
tap panel pages.graph
expect editor.map is graph
pentap map-node 7
expect editor.map.selected is 7
expect editor.map is graph
pentap map-node 7
expect editor.map is graph
expect editor.map.selected is 7
wait 600
pentap map-node 7
expect editor.map is 0
quit
