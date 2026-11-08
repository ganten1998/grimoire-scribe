# rust-only
# generate: tests/previews_fixture.py
# copy: tests/fixtures/scen-templates templates
# env: GRIMOIRE_KEEP_RENDER_MS=0
# env: GRIMOIRE_CACHE_WINDOW_MS=0
# A template this notebook has already (page 2's) picked for a page with a current kept drawing
# (z_keep_ghost_1_clear): its picture is there, so the page is drawn on it at once. 2.0.0 and 2.0.1 read the
# drawing kept of the file back instead, the old paper under the old ink, and kept that picture under the new
# file's stamp.
wait 300
tap library notebook.dense
expect editor.page is 1
tap panel next
expect editor.page is 2
pentap panel paper
expect editor.modal is paper
pentap panel paper.more
wait 300
expect editor.modal is templates
waitbg
wait 300
expect templates.ready is 2
pentap 511 452
wait 300
expect editor.modal is none
expect editor.paper_hex is 0x50f4d83c
waitbg
wait 300
expect editor.picture is shown
# Page 1, drawn as it opens (its picture made since: the previews' generation moved on) and kept so
tap panel previous
expect editor.page is 1
expect editor.paper is 0
expect editor.page_kept is 0
expect editor.kept_matches_render is 1
pentap panel paper
expect editor.modal is paper
pentap panel paper.more
wait 300
expect editor.modal is templates
waitbg
wait 300
expect templates.selected is
pentap 511 452
wait 300
expect editor.modal is none
expect editor.paper_hex is 0x50f4d83c
expect editor.strokes is 1000
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 01-on-lined
pen leave
wait 3000
expect editor.unsaved is 0
expect editor.kept_matches_render not 0
expect editor.preview_matches_render is 1
tap panel next
expect editor.page is 2
expect editor.card_matches_render is 1
tap panel previous
expect editor.page is 1
expect editor.paper_hex is 0x50f4d83c
expect editor.live_matches_render is 1
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect editor.paper_hex is 0x50f4d83c
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 1
quit
