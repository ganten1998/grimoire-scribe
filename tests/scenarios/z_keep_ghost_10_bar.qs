# rust-only
# generate: tests/previews_fixture.py ghosts
# A page entered from the Links bar comes from the bar's canvas, drawn from its strokes ahead of the tap, not drawn
# again as a page that opens is ("barghost": page 1 links to page 2, cleared under 2.0.0 or 2.0.1 with its drawing
# current, its Pages preview of the old ink; it opens on page 1). Staying on page 2 from the bar takes the old
# drawing away and makes the preview again from the page shown, and the Pages view shows it. Before, only a page
# that opened did: one stayed on from the bar (or through "Back to page N", the same step) kept the old preview.
wait 300
tap library notebook.barghost
expect editor.page is 1
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.1 is p2
tap 479 2218
wait 300
expect editor.bar is peek
expect editor.bar.peek is 2
expect editor.page is 1
tap 600 1100
wait 300
expect editor.bar is 0
expect editor.page is 2
expect editor.strokes is 0
expect editor.live_matches_render is 1
expect editor.preview_matches_render is 1
tap panel pages
expect editor.modal is pages
expect editor.pages.ink.2 is 0
shot 01-tile-2
quit
