# rust-only
# generate: tests/previews_fixture.py
# env: GRIMOIRE_CACHE_WINDOW_MS=0
# Clear page on a dense page whose drawing the previews job kept (z_previews_2_kept), at the editor's own keep
# threshold: the page comes up from that drawing, and Clear draws it from its strokes, none (z_keep_ghost_1_clear).
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
expect editor.kept_matches_render is 1
pentap panel clear
wait 300
expect editor.strokes is 0
expect editor.live_matches_render is 1
pen leave
wait 3000
expect editor.unsaved is 0
expect editor.kept_matches_render not 0
expect editor.preview_matches_render is 1
pentap panel next
wait 300
expect editor.page is 2
expect editor.card_matches_render is 1
pentap panel previous
wait 300
expect editor.page is 1
expect editor.strokes is 0
expect editor.live_matches_render is 1
pentap panel library
wait 600
expect screen is library
tap library notebook.previews
expect editor.page is 1
expect editor.strokes is 0
expect editor.live_matches_render is 1
expect editor.kept_matches_render not 0
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 1
quit
