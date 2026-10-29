# rust-only
# generate: tests/map_fixture.py 250
# env: GRIMOIRE_CHECK_FIT=1
wait 200
tap 326 608
key 37 0
tap editor bar.index
expect editor.map is graph
expect editor.map.nodes is 250
expect editor.map.tray is 249
shot 01-collapsed-tray
tap 500 2110
shot 02-expanded-tray
rotate 1
wait 100
shot 03-expanded-landscape
