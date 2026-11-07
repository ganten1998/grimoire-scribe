# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=0
# A new paper on a page with a current kept drawing (z_keep_ghost_1_clear): 2.0.0 and 2.0.1 read the drawing kept
# of the file back, the old paper under the old ink, and kept it again under the new file's stamp. The page is drawn
# on its new paper now, and kept so.
wait 300
tap library notebook.dense
expect editor.page is 1
expect editor.paper is 0
expect editor.kept_matches_render is 1
pentap panel paper
expect editor.modal is paper
pentap panel paper.3
expect editor.paper is 3
expect editor.strokes is 1000
expect editor.live_matches_render is 1
shot 01-sheet-music
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
expect editor.paper is 3
expect editor.live_matches_render is 1
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect editor.paper is 3
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 1
quit
