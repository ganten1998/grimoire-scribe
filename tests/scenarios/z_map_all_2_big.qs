# rust-only
# generate: tests/map_fixture.py --library 50x100
# env: GRIMOIRE_BG_DELAY_MS=1500
# A library of 50 notebooks of 100 pages for the Graph of all notebooks: the index's first check fills on idle
# ticks without holding the main loop up, the Graph opens without holding it up either (its open time is in the
# log: "graph: All notebooks open"), and its first full layout runs in a bgjob child, the seed shown meanwhile.
# The index is waited for, not a time: it reads a notebook an idle tick, after the backlinks cache's own check,
# and how fast the ticks come is the machine's (a slow CI runner had read 34 of the 50 in the 8 s this
# scenario once gave it). libindex.reads leaves 0 when the backlinks check is through and the index begins;
# libindex.complete is 1 when all 50 are read: 60 s for each.
wait 300
tap library folder.Library
tap library notebook.lib000
expect main.longest_unpumped_ms not -1
waitfor libindex.reads not 0
waitfor libindex.complete is 1
expect libindex.books is 50
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
