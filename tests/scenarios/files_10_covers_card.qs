# settings: status_mode=1
# settings: files_card=1
# settings: files_view=covers
# storage: 1
# generate: tests/files_place_tree.py
# copy: tests/fixtures/pdf/planner documents/Covers
# copy: tests/fixtures/pdf/junk documents/Covers
# copy: tests/fixtures/photos documents/Covers
# copy: tests/fixtures/fonts/good documents/Covers
# env: GRIMOIRE_BG_DELAY_MS=400
# Files' Covers view (2.2, issue #145): previews that land while an item card is open reach the grid when the card
# closes. A tile is tapped while the screen's previews are still being made (each job held back 400 ms here); the
# card stays open until all of them are made, and no tile is drawn under it. Closed, every tile shows what it has
# (no hourglass left), and the panel holds what the screen holds over the whole grid: the card's rect and the tiles
# that changed went out together.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 500
expect files.view is covers
expect files.tiles has l
tap library files.item.IMG_0001.jpg
wait 300
expect files.card is picture
waitfor files.making is 0
wait 2500
shot 01-card-over-the-tiles
tap library files.card.cancel
wait 300
expect files.card is
expect files.tiles is pipppppp
expect display.stale.46.356.1766.1937 is 0
shot 02-closed
wait 2500
expect display.stale.46.356.1766.1937 is 0
expect files.making is 0
quit
