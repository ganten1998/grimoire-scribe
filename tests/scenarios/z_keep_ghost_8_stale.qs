# rust-only
# generate: tests/previews_fixture.py ghosts
# A cleared page's old picture that 2.0.0 or 2.0.1 kept, then written on within 30 s of the Clear ("stale": their
# idle save inside the previews' 30 s wrote the page file alone, so their drawing is of the file as it was, while the
# Pages preview and the Library card hold the old ink and the new; it opens on page 1). Opened, the page is drawn
# from its strokes and the old drawing goes all the same: the preview is made again from the page drawn, and the
# card as page 1 is left. Before, only a drawing current for the page file went, and the card kept the old ink.
wait 300
tap library notebook.stale
expect editor.page is 1
expect editor.page_kept is 0
expect editor.live_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 0
tap panel library
wait 600
expect screen is library
shot 01-library
tap library notebook.stale
expect editor.page is 1
expect editor.card_matches_render is 1
expect editor.preview_matches_render is 1
quit
