# rust-only
# generate: tests/map_fixture.py --library 3x3
# env: GRIMOIRE_CHECK_FIT=1
# Back to the Library's Graph from further away (2.0.0-beta.2): a notebook the Graph opened keeps the way back
# across a link into another notebook (Back there returns to the first notebook, Back again to the Graph), and
# the Links bar's "Back to Graph" chip goes back to it too. In each notebook page 1 links to its page 2 and
# (area 300 1100 400 200) to the next notebook's page 2.
wait 300
key 50 0
wait 100
expect library.map is graph
wait 2500
pentap map-node lib000:1
wait 500
pentap map-node lib000:1
expect screen is editor
expect editor.notebook_id is lib000
expect editor.page is 1
pen leave
wait 1100
tap 500 1200
expect editor.notebook_id is lib001
expect editor.page is 2
# Back (key.b): the first notebook, then the Graph, the page that opened it selected.
key 48 0
wait 100
expect screen is editor
expect editor.notebook_id is lib000
expect editor.page is 1
key 48 0
wait 100
expect screen is library
expect library.map is graph
expect library.map.selected.id is lib000
# The Links bar's chip: Back to Graph.
pentap map-node lib000:1
expect screen is editor
expect editor.notebook_id is lib000
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.back is Back to Graph
tap panel bar.back
expect screen is library
expect library.map is graph
