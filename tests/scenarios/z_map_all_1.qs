# rust-only
# generate: tests/map_fixture.py --library 3x12
# env: GRIMOIRE_CHECK_FIT=1
# The Graph opens on All notebooks (2.0.0-beta.2, docs/beta2/GRAPH-ALL.md): every notebook's linked pages, links
# across notebooks its edges. In each of lib000, lib001 and lib002 page 1 links to its page 2 and to the next
# notebook's page 2, and page 3 to its Index. There is no Notebook choice (the owner's ruling of 2026-11-03,
# "remove it": the chip is gone); a node of another notebook opens it there, and Back comes back.
wait 300
tap library folder.Library
tap library notebook.lib000
expect editor.notebook_id is lib000
expect editor.page is 1
# (the library index's first check: idle ticks)
wait 1500
tap panel pages
tap panel pages.graph
expect editor.map is graph
expect editor.map.scope is all
expect editor.map.books is 3
expect editor.map.visible.ids is lib000:1 lib000:2 lib000:3 lib001:1 lib001:2 lib001:3 lib002:1 lib002:2 lib002:3
expect editor.map.edges is 6
expect editor.map.layout is done
expect editor.map.stats is 9 27 9 12 36
shot 01-all-notebooks
# Around crosses notebooks: lib000's page 1 links to lib001's page 2, and lib002's page 1 to lib000's page 2.
tap map-node 1
expect editor.map.selected is 1
expect editor.map.selected.id is lib000
shot 02-selected-rings
tap panel map.around
expect editor.map.mode is around
expect editor.map.visible.ids has lib001:2
expect editor.map.visible.ids has lib002:1
shot 03-around-across
tap panel map.view
expect editor.map.mode is all
# No Notebook choice (the owner's ruling of 2026-11-03, "remove it"): where the chip was, nothing.
expect editor.map.ui_at.400.215 is
tap 400 215
expect editor.map is graph
expect editor.map.scope is all
expect editor.map.mode is all
expect editor.destination is 0
shot 04-no-notebook-chip
# Through the Pages tab and back: every notebook still.
tap panel map.pages
expect editor.map is 0
tap panel pages.graph
expect editor.map.scope is all
expect editor.map.books is 3
# A node of another notebook: its card names the notebook; a second tap opens it there.
tap map-node lib001:2
expect editor.map.selected.id is lib001
expect editor.map.selected is 2
shot 07-other-card
tap map-node lib001:2
expect editor.map is 0
expect editor.notebook_id is lib001
expect editor.page is 2
expect editor.notebook_back is 1 lib000
key 48 0
expect editor.notebook_id is lib000
expect editor.page is 1
# The Graph closed: it opens on All notebooks again.
tap panel pages
tap panel pages.graph
expect editor.map.scope is all
rotate 1
wait 200
expect editor.map is graph
expect editor.screen_composed is 1
shot 08-all-landscape
rotate 0
wait 200
tap 100 95
expect editor.map is 0
