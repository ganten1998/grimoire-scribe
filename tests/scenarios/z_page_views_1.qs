# rust-only
# generate: tests/previews_fixture.py slow
# Drawings kept in memory (#49, issue #31): a page left this session and turned back to shows its drawing at
# once instead of being drawn again; a page written since is drawn again, never shown as it was. Page 1 is
# slow (381 KB, no drawing kept on disk), so drawing it says "Loading page..."; from memory it does not.
clock 1700000000
wait 3600
tap library notebook.slow
expect editor.page is 2
expect editor.page_in_memory is 0
# page 1, first time: drawn, with the toast
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_in_memory is 0
expect editor.loading_toasts is 1
# page 2 again: left a moment ago, from memory
pentap panel next
wait 300
expect editor.page is 2
expect editor.page_in_memory is 1
expect editor.page_kept is 0
expect editor.live_matches_render is 1
# page 1 again: from memory, so no second toast, and the drawing is the page's
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_in_memory is 1
expect editor.loading_toasts is 1
expect editor.live_matches_render is 1
# written on, left and turned back to: what shows is the page with the new stroke, not the drawing from before
pen line 400 900 1000 910 40 0.6 0.6
wait 300
pentap panel next
wait 300
expect editor.page is 2
pentap panel previous
wait 300
expect editor.page is 1
expect editor.live_matches_render is 1
quit
