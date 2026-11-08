# rust-only
# generate: tests/previews_fixture.py
# copy: tests/fixtures/scen-templates templates
# env: GRIMOIRE_KEEP_RENDER_MS=4294967294
# The new paper of a dense page (a template or a photo the notebook already has), the page left at once, before
# any idle save: the drawing it keeps as it is left is of the page on its new paper, never the old one, and so are
# the Library card and the drawing read back at the next open.
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
tap panel previous
expect editor.page is 1
expect editor.paper is 0
expect editor.page_kept is 0
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
pen leave
tap panel next
expect editor.page is 2
tap panel previous
expect editor.page is 1
expect editor.paper_hex is 0x50f4d83c
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
tap panel library
wait 600
expect screen is library
tap library notebook.dense
expect editor.page is 1
expect editor.paper_hex is 0x50f4d83c
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
expect editor.card_matches_render is 1
quit
