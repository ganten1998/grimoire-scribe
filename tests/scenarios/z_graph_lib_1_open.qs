# rust-only
# generate: tests/map_fixture.py --library 3x3
# env: GRIMOIRE_CHECK_FIT=1
# The Graph as a place of its own (2.0.0-beta.2, the owner's ruling of 2026-11-03; docs/beta2/GRAPH-ALL.md
# section 10): two fingers down over the Library open it on every notebook, with no notebook open: no List
# or Pages tab, and no Notebook choice (the owner's ruling of 2026-11-03, "remove it"). A node's second tap
# opens that notebook at that page; Back from there comes back to the Graph, the page selected; Back there
# returns to the Library. In each of lib000, lib001 and lib002 page 1 links to its page 2 and to the next
# notebook's page 2, and page 3 to its Index.
wait 300
expect screen is library
swipe 2 930 900 0 400
expect screen is library
expect library.map is graph
expect library.map.no_notebook is 1
expect library.map.scope is all
expect library.map.tabs is graph
# (the library index's first check runs on the Graph's quiet ticks: no notebook has been opened)
wait 2500
expect library.map.books is 3
expect library.map.visible.ids is lib000:1 lib000:2 lib000:3 lib001:1 lib001:2 lib001:3 lib002:1 lib002:2 lib002:3
expect library.map.edges is 6
shot 01-graph-from-library
# No Notebook chip: where it was, nothing, and a tap there opens no browser.
expect library.map.ui_at.400.215 is
tap 400 215
expect library.map is graph
expect library.map.picker is 0
expect library.map.scope is all
# A node: its card names its notebook; a second tap opens it there.
tap map-node lib001:2
expect library.map.selected.id is lib001
expect library.map.selected is 2
shot 04-card
tap map-node lib001:2
expect screen is editor
expect editor.notebook_id is lib001
expect editor.page is 2
expect library.map.keep is 1
# Back (nothing before it in this notebook): the Graph again, the page selected.
key 48 0
expect screen is library
expect library.map is graph
expect library.map.no_notebook is 1
expect library.map.scope is all
expect library.map.selected.id is lib001
expect library.map.selected is 2
shot 05-back-to-graph
# Back over the Graph: the Library.
tap library map.back
expect screen is library
expect library.map is 0
expect library.map.keep is 0
expect library.sheet is none
shot 06-library
