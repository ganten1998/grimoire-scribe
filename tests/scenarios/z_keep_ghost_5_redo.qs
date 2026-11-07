# rust-only
# generate: tests/previews_fixture.py linked
# env: GRIMOIRE_KEEP_RENDER_MS=0
# Redo of a Clear that took a link away, on a page with a current kept drawing (z_keep_ghost_1_clear): that redo
# draws the page whole again (undo_redo_link), and 2.0.0 and 2.0.1 read the drawing kept of the file back there
# too, the page marked changed already: nothing is saved in between here, so the file still has the old ink.
wait 300
tap library notebook.dense
expect editor.page is 1
expect editor.links is 1 1
expect editor.kept_matches_render is 1
tap panel clear
expect editor.strokes is 0
expect editor.links is 0 0
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.strokes is 1000
expect editor.links is 1 1
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.redo_kind is
expect editor.strokes is 0
expect editor.links is 0 0
expect editor.live_matches_render is 1
shot 01-redone
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
expect editor.strokes is 0
expect editor.live_matches_render is 1
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect editor.strokes is 0
expect editor.links is 0 0
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 1
quit
