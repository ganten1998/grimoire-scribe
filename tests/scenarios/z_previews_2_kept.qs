# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=0
# The previews job keeps the drawing of a dense page it draws (2.0.0-beta.2: a page file of 512 KB or more, as
# "Loading page..." counts one), so the dense page then opens from it at once: read back, not drawn, and no
# "Loading page...". A light page keeps none from the job, however slow (the owner's ruling: ordinary pages save
# nothing extra; GRIMOIRE_KEEP_RENDER_MS=0 counts every drawing as slow, as the editor counts them), and opens
# drawn, as before.
wait 300
tap library notebook.previews
expect editor.page is 2
tap panel pages
expect editor.modal is pages
waitbg 30000
wait 300
expect editor.pages.shows is ppppppppp
tap panel pages.back
expect editor.modal is none
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_kept is 1
expect editor.loading_toasts is 0
expect editor.live_matches_render is 1
# page 3, light, its preview made by the same batch: drawn as it opens, no drawing kept by the job
pentap panel next
wait 300
pentap panel next
wait 300
expect editor.page is 3
expect editor.page_kept is 0
expect editor.loading_toasts is 0
quit
