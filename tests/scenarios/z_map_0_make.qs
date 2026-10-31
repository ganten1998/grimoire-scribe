# rust-only
# generate: tests/map_fixture.py 40
# env: GRIMOIRE_CHECK_FIT=1
wait 300
expect library.items is 1
tap 326 608
expect editor.pages is 40
expect editor.page is 3
tap panel pages
tap panel pages.graph
expect editor.map is graph
expect editor.map.nodes is 40
# Only the 23 linked pages are laid out: the unlinked ones have no tray and take no room.
expect editor.map.force is 23
expect editor.map.tray is 0
expect editor.map.layout is done
shot 01-forty-pages
tap map-node 7
expect editor.map.selected is 7
shot 02-selected
tap 100 95
expect editor.map is 0
pentap panel library
waitbg
