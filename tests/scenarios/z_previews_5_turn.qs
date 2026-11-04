# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_SIM_DENSE_DRAW_MS=3000 GRIMOIRE_SIM_CORES=1
# The Pages view turned to its next screen while the first screen's previews are still being made (a dense page
# among them, as slow as on a Kindle): the new screen's previews are asked for at once and come after that batch
# (with one background process: GRIMOIRE_SIM_CORES=1). Before 2.0.0-beta.2 they were never asked for, and its
# tiles kept their numbers.
wait 300
tap library notebook.previews
tap panel pages
expect editor.modal is pages
wait 300
expect bg.busy is 1
fpath 1400 1300 600 1320 200
wait 300
expect editor.pages.screen is 2
expect editor.pages.shows is lll
waitbg 30000
wait 400
expect editor.pages.shows is ppp
quit
