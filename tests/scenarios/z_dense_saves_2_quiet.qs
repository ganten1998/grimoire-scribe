# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=4294967294
# env: GRIMOIRE_CACHE_WINDOW_MS=4000
# A dense page written from blank writes its kept drawing at an idle save only once the page has been quiet for the
# window (30 s on the Kindle): written in bursts, it would write its whole drawing (3.4 MB) at every pause, and
# leaving writes it anyway. The Pages preview is made at the idle save as before.
wait 300
tap library notebook.dense
expect editor.page is 1
pen line 400 900 1000 910 40 0.6 0.6
pen leave
wait 3000
expect editor.unsaved is 0
expect editor.preview_matches_render is 1
expect editor.kept_matches_render is none
# A second burst inside the quiet time: still none
pen line 400 1000 1000 1010 40 0.6 0.6
pen leave
wait 3500
expect editor.unsaved is 0
expect editor.kept_matches_render is none
# Quiet for the window: the drawing is made, with nothing written to the page file
wait 7000
expect editor.kept_matches_render is 1
quit
