# rust-only
# generate: tests/map_fixture.py --library 3x3
# env: GRIMOIRE_CHECK_FIT=1
# The Library's Graph on a keyboard: the
# arrows pan it, V switches View: All and View: Adjacent, Enter opens the one page selected in its notebook, and
# Close menu (key.x in tests/map_fixture.py's shortcuts.ini) closes it. Up/Down do nothing in the Library itself.
wait 300
key 103 0
expect library.map is 0
key 50 0
wait 100
expect library.map is graph
wait 2500
expect library.map.books is 3
expect library.map.view is all
key 106 0
expect library.map is graph
expect library.map.touched is 1
key 47 0
expect library.map.view is adjacent
expect library.map.touched is 0
key 47 0
expect library.map.view is all
tap map-node lib001:1
expect library.map.selected.id is lib001
key 28 0
expect screen is editor
expect editor.notebook_id is lib001
expect editor.page is 1
tap panel library
expect screen is library
expect library.map is graph
key 45 0
expect library.map is 0
quit
