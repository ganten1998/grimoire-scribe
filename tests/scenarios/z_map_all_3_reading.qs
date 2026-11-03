# rust-only
# generate: tests/map_fixture.py --library 120x3
# env: GRIMOIRE_CHECK_FIT=1
# The Graph of all notebooks opened before the library index's first check is through (120 notebooks, one read
# a tick after the backlinks cache's own check): the panel's counts line says how far it has got, and the notebooks join as
# they are read, every kept dot staying where it was.
wait 300
tap library folder.Library
tap library notebook.lib000
tap panel pages
tap panel pages.graph
expect editor.map is graph
expect editor.map.scope is all
expect editor.map.building is 1
shot 01-reading-notebooks
wait 20000
expect editor.map.building is 0
expect editor.map.books is 120
expect editor.map.visible.ids has lib119:2
shot 02-read
tap 100 95
expect editor.map is 0
