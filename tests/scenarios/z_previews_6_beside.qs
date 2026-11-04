# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_SIM_DENSE_DRAW_MS=6000 GRIMOIRE_SIM_CORES=4
# With the cores for it (three or more; GRIMOIRE_SIM_CORES=4 here), the next screen's previews are made by a
# second background process beside the first, still busy with the dense page (2.0.0-beta.2): they show while
# that one is still being drawn.
wait 300
tap library notebook.previews
tap panel pages
expect editor.modal is pages
wait 300
expect bg.busy is 1
fpath 1400 1300 600 1320 200
wait 1200
expect editor.pages.screen is 2
expect bg.busy is 1
expect editor.pages.shows is ppp
waitbg 30000
fpath 600 1300 1400 1320 200
wait 400
expect editor.pages.screen is 1
expect editor.pages.shows is ppppppppp
quit
