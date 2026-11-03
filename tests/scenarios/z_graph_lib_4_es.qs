# rust-only
# generate: tests/map_fixture.py --library 3x3
# env: GRIMOIRE_CHECK_FIT=1 GRIMOIRE_LANG=es
# The Graph from the Library and Leave out of the Graph in Spanish (2.0.0-beta.2): the top bar (its five
# buttons: no Graph button, the owner's ruling of 2026-11-03), a notebook's and a folder's menus with their rows (and the greyed-out row of a notebook whose folder
# is left out), the toasts, and the Graph opened from the Library (its one tab, its panel's library line), each without a text cut short.
wait 300
shot 01-library
tap library folder-menu.Library
expect library.sheet is folder-more
shot 02-folder-menu
tap library done
tap library folder.Library
tap library notebook-menu.lib001
shot 03-notebook-menu
tap library more.graph
expect toast is Fuera del grafo.
shot 04-toast
tap library notebook-menu.lib001
shot 05-show-row
tap library more.graph
expect toast is De nuevo en el grafo.
tap library back
tap library folder-menu.Library
tap library fmore.graph
tap library folder.Library
tap library notebook-menu.lib000
shot 06-row-with-its-folder
tap library done
tap library back
tap library folder-menu.Library
tap library fmore.graph
swipe 2 930 900 0 400
expect library.map is graph
wait 2500
expect library.map.books is 3
shot 07-graph
# The Links bar of a notebook the Graph opened: its chip back to the Graph, in words of its own ("Volver al
# grafo", not the shared "Volver a %s" filled with "Grafo").
pentap map-node lib000:1
wait 500
pentap map-node lib000:1
expect screen is editor
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
expect editor.bar.back is Volver al grafo
shot 09-bar-back-to-graph
