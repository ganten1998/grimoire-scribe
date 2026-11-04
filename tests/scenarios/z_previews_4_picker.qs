# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_SIM_DENSE_DRAW_MS=4000 GRIMOIRE_SIM_CORES=1
# The destination picker's pages (Copy to notebook..., Link to...'s other notebooks) take their previews as the
# Pages view does (2.0.0-beta.2): each tile as its preview lands, the dense page (page 9 of "other") last, its
# tile showing the loading mark until then, and that tile alone refreshed as its preview comes.
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library notebook.previews
tap panel pages
expect editor.modal is pages
waitbg 30000
tap panel pages.page.3
expect editor.ticked is 1
tap panel pages.copy-to-notebook
expect editor.destination is 1
tap panel elsewhere.book.other
expect editor.destination.notebook is other
wait 1500
expect bg.busy is 1
expect editor.destination.shows is pppppppplp
shot 01-loading
expect display.presents not -1
waitbg 30000
wait 400
expect editor.destination.shows is pppppppppp
expect display.presents is ui 35 900 421 325
shot 02-ready
quit
