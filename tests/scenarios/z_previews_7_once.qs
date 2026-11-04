# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_SIM_DENSE_DRAW_MS=6000 GRIMOIRE_SIM_CORES=4
# The Pages view opened while the idle warm-up still makes the previews of the open page's screenful (the dense page 1
# among them, as slow as on a Kindle), page 2's own preview current (its idle save wrote it): the view waits on that
# batch, the very one it would ask for, and no second background process starts on the same pages beside it, though
# the cores are there for one (GRIMOIRE_SIM_CORES=4): it would draw the dense page a second time (2.0.0-beta.2).
wait 300
tap library notebook.previews
expect editor.page is 2
pen line 600 900 1200 920 25 0.6 0.6
pen leave
wait 3000
# (page 2 saved, its preview written; the warm-up drawing page 1)
expect bg.children is 1
tap panel pages
expect editor.modal is pages
wait 600
expect bg.children is 1
expect editor.pages.shows is lpppppppp
waitbg 30000
wait 400
expect editor.pages.shows is ppppppppp
quit
