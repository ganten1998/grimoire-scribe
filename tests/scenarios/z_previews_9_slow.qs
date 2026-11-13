# rust-only
# generate: tests/previews_fixture.py slow
# env: GRIMOIRE_KEEP_RENDER_MS=0
# env: GRIMOIRE_PAGE_VIEWS=0
# (No drawings kept in memory: a revisit reads the drawing kept on disk, as one in another session does.)
# A slow page under 512 KB (2.0.0-beta.2, the owner's decision of 2026-11-04: "Loading page..." from 256 KB,
# kept drawings still from 512 KB): the previews job makes its preview and keeps no drawing of it, so opening it
# says "Loading page..." on the panel before the drawing starts, gone once the page shows. A light page under
# 256 KB with no kept drawing says nothing. (GRIMOIRE_KEEP_RENDER_MS=0 keeps every drawing the editor makes, as it
# keeps one of a page it drew slowly: the slow page's at its first open, so it then opens from that, quickly.)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library notebook.slow
expect editor.page is 2
expect editor.loading_toasts is 0
# the previews of pages 1 and 3 (page 2's from its canvas): no drawing kept by the job for page 1
tap panel pages
expect editor.modal is pages
waitbg 30000
wait 300
expect editor.pages.shows is ppp
tap panel pages.back
expect editor.modal is none
wait 1500
expect display.presents not -1
# page 1, 381 KB, no kept drawing: the toast (with the button's release) in a refresh of its own, then the page
# (the button's press shows first, a fast refresh of it)
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_kept is 0
expect editor.loading_toasts is 1
expect toast is
# (a page turn at the fitted view is a UI refresh, no flash: #49, issue #31; every 30th UI refresh flashes)
expect display.presents is fast 1718 1104 100 100, ui 716 1104 1102 1256, ui 0 0 1860 2480
# page 3, light, never opened (no kept drawing): none
pentap panel next
wait 300
pentap panel next
wait 300
expect editor.page is 3
expect editor.page_kept is 0
expect editor.loading_toasts is 1
# page 1 again: from the drawing the editor kept as it opened, quickly, so none
pentap panel previous
wait 300
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_kept is 1
expect editor.loading_toasts is 1
quit
