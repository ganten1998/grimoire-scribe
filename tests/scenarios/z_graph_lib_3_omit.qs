# rust-only
# generate: tests/map_fixture.py --library 3x3
# env: GRIMOIRE_CHECK_FIT=1
# Leave out of the Graph (2.0.0-beta.2, the owner's ruling of 2026-11-03): a notebook's menu leaves it out of
# the Graph of all notebooks (no node of it; a page linked only with it is not linked there), and Show in the
# Graph brings it back; a folder's menu leaves out everything
# in it, and its notebooks' rows say so, greyed out. The menu row says which way it is, with no toast. lib000 page 1 links to lib000:2 and lib001:2, lib001 page 1
# to lib001:2 and lib002:2, lib002 page 1 to lib002:2 and lib000:2; each page 3 to its Index.
wait 300
tap library folder.Library
tap library notebook-menu.lib001
expect library.sheet is more
shot 01-notebook-menu
tap library more.graph
expect library.sheet is none
expect toast is
expect library.graph.omitted is lib001
swipe 2 930 900 0 400
expect library.map is graph
wait 2500
expect library.map.visible.ids is lib000:1 lib000:2 lib000:3 lib002:1 lib002:2 lib002:3
expect library.map.edges is 3
# (6 nodes, no orphan: the Graph's 6 pages are all linked; 3 links between pages and 2 to an Index)
expect library.map.stats is 6 0 5 0 6
shot 02-graph-without-lib001
tap library map.back
# Its own notebook: the Graph leaves it out there too (there is no Notebook choice to show it alone).
tap library notebook.lib001
expect editor.notebook_id is lib001
tap panel pages
tap panel pages.graph
expect editor.map.scope is all
expect editor.map.visible.ids is lib000:1 lib000:2 lib000:3 lib002:1 lib002:2 lib002:3
expect editor.map.ui_at.400.215 is
tap panel map.back
tap panel library
expect screen is library
# Show in the Graph: back.
tap library notebook-menu.lib001
tap library more.graph
expect toast is
expect library.graph.omitted is
# The folder: everything in it left out; a notebook's row in it greyed out.
tap library back
tap library folder-menu.Library
expect library.sheet is folder-more
tap library fmore.graph
expect toast is
expect library.graph.omitted is lib000 lib001 lib002
swipe 2 930 900 0 400
expect library.map is graph
wait 600
expect library.map.nodes is 0
shot 03-graph-folder-left-out
tap library map.back
tap library folder.Library
tap library notebook-menu.lib002
shot 04-row-with-its-folder
tap library done
tap library back
tap library folder-menu.Library
tap library fmore.graph
expect toast is
expect library.graph.omitted is
swipe 2 930 900 0 400
wait 600
expect library.map.books is 3
expect library.map.edges is 6
