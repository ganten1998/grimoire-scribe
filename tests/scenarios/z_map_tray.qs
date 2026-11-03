# rust-only
# generate: tests/map_fixture.py 250
# env: GRIMOIRE_CHECK_FIT=1
# A notebook without links has no tiny dots or unlinked tray in either orientation.
wait 200
tap 326 608
tap panel pages
tap panel pages.graph
expect editor.map is graph
# (the Graph is every notebook's: there is no Notebook choice, the owner's ruling of 2026-11-03)
expect editor.map.scope is all
expect editor.map.nodes is 0
expect editor.map.visible is
# (the library's page count comes from the Graph's library index, filled on idle ticks: its first check)
wait 1500
expect editor.map.stats is 0 250 0 250 250
shot 01-no-unlinked-tray
rotate 1
wait 100
expect editor.map.visible is
expect editor.screen_composed is 1
shot 02-no-unlinked-landscape
