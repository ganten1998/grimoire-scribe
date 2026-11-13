# env: GRIMOIRE_PAGE_VIEWS=0
# (No drawings kept in memory: a revisit reads the drawing kept on disk, as one in another session does.)
# settings: shortcuts_enabled=1
# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_KEEP_RENDER_MS=4294967294
# env: GRIMOIRE_CACHE_WINDOW_MS=60000
# A save that is not the idle one or the page being left (Ctrl+S, the Pages view; Settings, the power press and a
# sleep go the same way: save_page) writes the page file and no kept drawing: the drawing of a dense page is 3.4 MB
# (about 0.16 s on the Kindle), and a save at every Ctrl+S would write it at every one. Leaving the page writes it.
wait 300
tap library notebook.dense
expect editor.page is 1
expect editor.kept_matches_render is none
pen line 400 900 1000 910 40 0.6 0.6
pen leave
wait 400
key 31 1
wait 100
expect editor.unsaved is 0
expect editor.kept_matches_render is none
pen line 400 1000 1000 1010 40 0.6 0.6
pen leave
wait 400
key 25 0
wait 600
expect editor.modal is pages
expect editor.unsaved is 0
expect editor.kept_matches_render is none
key 1 0
wait 300
key 31 1
wait 100
expect editor.kept_matches_render is none
# Leaving writes it, inside the window as well
tap panel next
expect editor.page is 2
tap panel previous
expect editor.page is 1
expect editor.page_kept is 1
expect editor.live_matches_render is 1
expect editor.kept_matches_render is 1
quit
