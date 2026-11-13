# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=4294967294
# env: GRIMOIRE_PAGE_VIEWS=0
# (No drawings kept in memory: a revisit reads the drawing kept on disk, as one in another session does.)
# Leaving a dense page keeps its drawing at once (z_dense_first_revisit), so a Clear page followed by an
# immediate page turn, before any idle save, must keep the cleared page and never the ink it had: the drawing is
# written from the canvas, which Clear drew from the strokes (2.0.0 and 2.0.1 kept the old picture).
wait 300
tap library notebook.dense
expect editor.page is 1
pen line 400 900 1000 910 40 0.6 0.6
pentap panel next
expect editor.page is 2
pentap panel previous
expect editor.page is 1
expect editor.page_kept is 1
tap panel clear
expect editor.strokes is 0
expect editor.live_matches_render is 1
tap panel next
expect editor.page is 2
tap panel previous
expect editor.page is 1
expect editor.strokes is 0
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
expect editor.card_matches_render is 1
# and a Clear that is Undone, left at once
tap panel undo
expect editor.live_matches_render is 1
tap panel next
expect editor.page is 2
tap panel previous
expect editor.page is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
tap panel library
wait 600
tap library notebook.dense
expect editor.page is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
expect editor.card_matches_render is 1
quit
