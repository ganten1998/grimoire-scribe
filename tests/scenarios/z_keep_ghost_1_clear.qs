# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=0
# env: GRIMOIRE_CACHE_WINDOW_MS=0
# Clear page on a page with a current kept drawing (GRIMOIRE_KEEP_RENDER_MS=0: the dense page keeps its drawing
# as it first opens). A kept drawing is of the page file, which holds the old ink until the idle save: 2.0.0 and
# 2.0.1 read it back as Clear drew the emptied page again, so the cleared page showed its old picture, and the
# idle save kept that picture, and made the Pages preview and the Library card from it, under the cleared file's
# stamp, so it came back at every open. The page is drawn from its strokes now, and what is made from it after
# shows the page as it is.
wait 300
tap library notebook.dense
expect editor.page is 1
expect editor.page_kept is 0
expect editor.kept_matches_render is 1
tap panel clear
expect editor.strokes is 0
expect editor.live_matches_render is 1
shot 01-cleared
# The idle save (2 s after the change): the page, and its Pages preview from the canvas; no drawing of the old ink
wait 3000
expect editor.unsaved is 0
expect editor.kept_matches_render not 0
expect editor.preview_matches_render is 1
# Leaving page 1 writes the Library card from the canvas; back, the page is drawn as it is
tap panel next
expect editor.page is 2
expect editor.card_matches_render is 1
tap panel previous
expect editor.page is 1
expect editor.strokes is 0
expect editor.live_matches_render is 1
# And opened again: from the drawing kept of the cleared page
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect editor.strokes is 0
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 1
shot 02-opened-again
quit
