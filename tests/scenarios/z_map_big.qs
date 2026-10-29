# rust-only
# generate: tests/map_fixture.py 2000
# env: GRIMOIRE_BG_DELAY_MS=1500
wait 300
tap 326 608
expect editor.pages is 2000
key 37 0
tap 1590 2370
expect editor.map is graph
expect editor.map.nodes is 2000
expect editor.map.force is 2000
expect editor.map.edges is 4000
expect editor.map.layout is running
expect main.longest_unpumped_ms is ok
shot 01-seed
waitbg 120000
wait 600
expect editor.map.layout is done
expect editor.map.nodes is 2000
shot 02-done
