# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_SIM_DENSE_DRAW_MS=5000 GRIMOIRE_SIM_CORES=1
# A page deleted while the screenful's previews are still being made (the dense page 1, as slow as on a Kindle), and
# the view closed and opened again before that batch ends: the batch read the page list as it started, so the page
# that moved into the screenful (the tenth, ninth now) is not among its pages. It is asked for again, its tile waits
# for it, and then shows its preview (2.0.0-beta.2). Before, the view took the running batch for its own: that tile
# lost its loading mark as the batch ended, and kept its number while the view stayed open.
wait 300
tap library notebook.previews
tap panel pages
expect editor.modal is pages
wait 300
expect bg.busy is 1
# (page 3's tile held: its card's Delete)
touch down 1542 500
wait 700
touch up 1542 500
expect editor.pages.context is 3
tap panel pages.delete
wait 300
expect editor.pages is 11
tap panel pages.back
expect editor.modal is none
tap panel pages
expect editor.modal is pages
wait 300
expect bg.busy is 1
expect editor.pages.shows is lpppppppl
waitbg 30000
wait 400
expect editor.pages.shows is ppppppppp
quit
