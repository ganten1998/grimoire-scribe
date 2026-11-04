# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=0
# Opening a dense page with no kept drawing to read back (2.0.0-beta.2): "Loading page..." reaches the panel
# before the drawing starts (its own refresh, ahead of the page's), and is gone once the page shows. A light
# page never shows it, nor does the dense page once it opens from its kept drawing (GRIMOIRE_KEEP_RENDER_MS=0
# keeps every drawing, here the dense page's at its first open).
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
# A notebook that opens on its dense page: the toast over the Library first (with the card's release), then the
# page (the card's press shows first, a fast refresh of it)
expect display.presents not -1
tap library notebook.dense
wait 300
expect editor.page is 1
expect editor.loading_toasts is 1
expect toast is
expect display.presents is fast 50 264 553 737, ui 50 264 1093 2096, full 0 0 1860 2480
pentap panel library
wait 1500
tap library notebook.previews
expect editor.page is 2
expect editor.loading_toasts is 1
expect display.presents not -1
# page 1, dense, never drawn before: the toast (with the button's release) in a refresh of its own, then the
# page (the button's press shows first, a fast refresh of it)
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_kept is 0
expect editor.loading_toasts is 2
expect toast is
expect display.presents is fast 1718 1104 100 100, ui 716 1104 1102 1256, full 0 0 1860 2480
# page 2, light: none
pentap panel next
wait 300
expect editor.page is 2
expect editor.loading_toasts is 2
expect display.presents is fast 1718 1280 100 100, full 0 0 1860 2480
# page 1 again: from the drawing it kept as it opened, quickly, so none
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_kept is 1
expect editor.loading_toasts is 2
expect toast is
expect display.presents is fast 1718 1104 100 100, full 0 0 1860 2480
quit
