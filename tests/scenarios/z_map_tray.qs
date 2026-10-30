# rust-only
# generate: tests/map_fixture.py 250
# env: GRIMOIRE_CHECK_FIT=1
# A notebook without links has no tiny dots or unlinked tray in either orientation.
wait 200
tap 326 608
tap panel pages
tap panel pages.graph
expect editor.map is graph
expect editor.map.nodes is 250
expect editor.map.visible is
expect editor.map.stats is 0 250 0 250 250
shot 01-no-unlinked-tray
rotate 1
wait 100
expect editor.map.visible is
expect editor.screen_composed is 1
shot 02-no-unlinked-landscape
