# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=4294967294
# No slow opening to qualify this page: density at save time is sufficient. Leaving before idle
# must keep the final canvas; an immediate first revisit reads it, with exactly the new ink.
wait 300
tap library notebook.dense
expect editor.page_kept is 0
pen line 400 900 1000 910 40 0.6 0.6
pentap panel next
expect editor.page is 2
pentap panel previous
expect editor.page is 1
expect editor.page_kept is 1
expect editor.live_matches_render is 1
quit
