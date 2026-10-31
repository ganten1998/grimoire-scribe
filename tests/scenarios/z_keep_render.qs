# settings: status_mode=1
# settings: mirror_pdf=0
# env: GRIMOIRE_KEEP_RENDER_MS=0
# rust-only
# A page that takes long to draw keeps its drawing and opens from it (GRIMOIRE_KEEP_RENDER_MS=0 here
# makes every page count as slow): drawn the first time, kept at the save, read back at the next open,
# exactly what drawing it again gives (live_matches_render), and drawn again once its file changes.
wait 300
tap 965 95
wait 100
tap library paper.0
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.page_kept is 0
pen curve 300 700 40 80 0.7 4
pen line 300 900 1300 900 80 0.7 0.7
side on
pen line 280 880 1300 910 60 0.5 0.5
side off
wait 700
expect editor.strokes is 3
expect editor.live_matches_render is 1
pentap panel library
wait 1500
expect screen is library
tap 326 608
wait 600
expect screen is editor
expect editor.page_kept is 1
expect editor.strokes is 3
expect editor.live_matches_render is 1
# A new stroke saved: the next open is from the drawing kept at that save
pen line 400 1400 1200 1450 50 0.8 0.8
wait 700
pentap panel library
wait 1500
tap 326 608
wait 600
expect editor.page_kept is 1
expect editor.strokes is 4
expect editor.live_matches_render is 1
quit
