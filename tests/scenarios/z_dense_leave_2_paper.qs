# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=4294967294
# env: GRIMOIRE_PAGE_VIEWS=0
# (No drawings kept in memory: a revisit reads the drawing kept on disk, as one in another session does.)
# A new paper on a dense page, then the notebook closed at once (the drawing is written as the page is left, before
# any idle save): the notebook opens on its new paper, and the drawing kept, the Pages preview and the Library card
# are of the page as it is, never of the old paper.
wait 300
tap library notebook.dense
expect editor.page is 1
expect editor.paper is 0
pen line 400 900 1000 910 40 0.6 0.6
pentap panel next
pentap panel previous
expect editor.page_kept is 1
pentap panel paper
expect editor.modal is paper
pentap panel paper.3
expect editor.paper is 3
expect editor.live_matches_render is 1
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect editor.paper is 3
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
expect editor.card_matches_render is 1
quit
