# rust-only
# generate: tests/map_fixture.py --library 50x100
# env: GRIMOIRE_BG_DELAY_MS=1500
# A library of 50 notebooks of 100 pages for the Graph of all notebooks: the index's first check fills on idle
# ticks without holding the main loop up, the Graph opens without holding it up either (its open time is in the
# log: "graph: All notebooks open"), and its first full layout runs in a bgjob child, the seed shown meanwhile.
wait 300
tap library folder.Library
tap library notebook.lib000
expect main.longest_unpumped_ms not -1
wait 8000
expect main.longest_unpumped_ms is ok
expect editor.notebook_id is lib000
tap panel pages
expect main.longest_unpumped_ms not -1
tap panel pages.graph
expect main.longest_unpumped_ms is ok
expect editor.map is graph
expect editor.map.scope is all
expect editor.map.books is 50
expect editor.map.layout is running
shot 01-seed
waitbg 120000
wait 600
expect editor.map.layout is done
expect editor.map.books is 50
shot 02-laid-out
tap 100 95
expect editor.map is 0
