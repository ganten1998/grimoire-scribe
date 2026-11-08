# rust-only
# generate: tests/previews_fixture.py
# copy: tests/fixtures/photos photos
# env: GRIMOIRE_KEEP_RENDER_MS=0
# env: GRIMOIRE_CACHE_WINDOW_MS=0
# A photo this notebook has already (page 2's) picked for a page with a current kept drawing (z_keep_ghost_3_template):
# its picture is there, so the page goes onto it at once. 2.0.0 and 2.0.1 read the drawing kept of the file back
# instead, the old paper under the old ink, and kept that picture under the new file's stamp. (A photo new to the
# notebook moves the previews' generation on as its picture is made, which made that drawing stale: there the old
# picture showed only until the photo's was made.)
wait 300
tap library notebook.dense
expect editor.page is 1
tap panel next
expect editor.page is 2
pentap panel paper
expect editor.modal is paper
pentap panel paper.photos
wait 300
expect editor.modal is templates
waitbg
wait 300
expect templates.ready is 2
pentap 232 452
wait 300
expect editor.modal is none
expect annotate.role is document
waitbg
wait 300
expect editor.picture is shown
# Page 1, drawn as it opens (the previews' generation moved on) and kept so
tap panel previous
expect editor.page is 1
expect editor.page_kept is 0
expect editor.kept_matches_render is 1
pentap panel paper
expect editor.modal is paper
pentap panel paper.photos
wait 300
expect editor.modal is templates
waitbg
wait 300
pentap 232 452
wait 300
expect editor.modal is none
expect annotate.role is document
expect editor.strokes is 1000
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 01-on-the-photo
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
expect annotate.role is document
expect editor.live_matches_render is 1
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect annotate.role is document
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 1
quit
