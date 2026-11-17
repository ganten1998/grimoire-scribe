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
# Files' Covers view with a picture's card (2.2, issues #145 and #146): a tap on a picture's tile opens the card the
# list's row opens, with Use as template and Use as photo. Previews that land while the card is open reach the grid
# when Use as template closes it, as when Cancel does: the card's rect and the tiles that changed go out together,
# so the panel holds what the screen holds (the rect checked is the grid above the toast's place). The copy is
# made, and said, as from the list.
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
expect files.card.rows is Use as template|Use as photo|Cancel
waitfor files.making is 0
wait 2500
shot 01-card-over-the-tiles
tap library files.card.template
wait 300
expect files.card is
expect files.tiles is pipppppp
expect display.stale.46.356.1766.1840 is 0
waitbg
wait 200
expect toast is Copied to Templates.
expect storage.names.grimoire/templates is IMG_0001.jpg|README.txt
expect files.place is documents/Covers
expect display.stale.46.356.1766.1840 is 0
shot 02-copied
wait 2500
expect display.stale.46.356.1766.1840 is 0
expect files.making is 0
quit
