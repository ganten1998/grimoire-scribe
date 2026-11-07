# rust-only
# generate: tests/previews_fixture.py ghosts
# The same with page 1 dense, its preview not current ("stale2", opens on page 2): the previews job draws page 1 and
# makes its preview, but keeps no drawing over the one 2.0.0 or 2.0.1 kept, so page 1, opened, is drawn from its
# strokes, the old drawing goes and the Library card is made again as page 1 is left. Before, the job kept its own
# drawing over the old one, the page came up from it, and the card kept the old ink.
wait 300
tap library notebook.stale2
expect editor.page is 2
tap panel pages
expect editor.modal is pages
waitbg 30000
wait 300
expect editor.pages.ink.1 not none
tap panel pages.back
expect editor.modal is none
pentap panel previous
wait 300
expect editor.page is 1
expect editor.page_kept is 0
expect editor.live_matches_render is 1
expect editor.preview_matches_render is 1
expect editor.card_matches_render is 0
pentap panel library
wait 600
expect screen is library
tap library notebook.stale2
expect editor.page is 1
expect editor.card_matches_render is 1
quit
