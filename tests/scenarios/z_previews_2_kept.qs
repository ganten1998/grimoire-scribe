# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=0
# The previews job keeps the drawing of a page it drew slowly (2.0.0-beta.2; GRIMOIRE_KEEP_RENDER_MS=0 counts
# every page as slow), so the dense page then opens from it at once: read back, not drawn, and no
# "Loading page...".
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
quit
