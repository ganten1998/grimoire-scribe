# rust-only
# generate: tests/map_fixture.py --library 3x3
# env: GRIMOIRE_CHECK_FIT=1
# The Graph from the Library with a keyboard: the index-map key (key.m in tests/map_fixture.py's shortcuts.ini)
# opens it and closes it (the top bar has no Graph button: the owner's ruling of 2026-11-03, "For now just the
# drag to summon on the main menu"; where a sixth button would be, left of New, the bar is empty), a second pen
# tap on a node opens it (past PEN_REOPEN_MS: a sooner one is a stylus bounce), the Library button of a
# notebook it opened comes back to it, and go-back (key.b) over it returns to the Library.
wait 300
key 50 0
wait 100
expect library.map is graph
expect library.map.tabs is graph
key 50 0
wait 100
expect library.map is 0
expect library.sheet is none
expect library.ui_at.770.95 is
key 50 0
wait 100
expect library.map is graph
expect library.map.no_notebook is 1
wait 2500
expect library.map.books is 3
pentap map-node lib000:1
wait 600
pentap map-node lib000:1
expect screen is editor
expect editor.notebook_id is lib000
expect editor.page is 1
# The toolbar's Library button closes the notebook into the Library as it was left: the Graph.
tap panel library
expect screen is library
expect library.map is graph
expect library.map.selected.id is lib000
expect library.map.selected is 1
# go-back over the Graph: the Library.
key 48 0
wait 100
expect library.map is 0
expect screen is library
expect library.sheet is none
# A notebook opened from its card is not the Graph's: its Library button goes to the Library.
tap library folder.Library
tap library notebook.lib002
expect screen is editor
tap panel library
expect screen is library
expect library.map is 0
