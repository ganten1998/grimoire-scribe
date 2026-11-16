# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# copy: tests/fixtures/pdf/planner documents/Covers
# copy: tests/fixtures/pdf/junk documents/Covers
# copy: tests/fixtures/photos documents/Covers
# copy: tests/fixtures/fonts/good documents/Covers
# env: GRIMOIRE_BG_DELAY_MS=400
# Files' Covers view (2.2, issue #145): documents/Covers holds a planner PDF, a PDF that is no PDF, two photos and
# four fonts. The View chip takes the folder from its list to a grid of covers: each tile shows the hourglass while
# its file's preview is made by the sandboxed worker (each job held back 400 ms here, as a Kindle takes its time),
# "Making previews..." stays meanwhile, and the previews arrive tile by tile; the PDF the worker refuses shows its
# kind's icon. A tap on a tile opens its file's card, as a row's does. Back to the list, then out of Files and in
# again: the view chosen stays (files_view), and the previews made are not made again.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 300
expect files.view is list
expect files.rows is 8
expect files.tiles is
tap library files.view
wait 100
expect files.view is covers
expect files.per_page is 9
waitfor notice.key is previews
expect notice is Making previews...
expect files.making is 1
expect files.tiles has l
shot 01-making
waitfor files.making is 0
waitfor notice is
expect files.asked is 8
expect files.tiles is pipppppp
shot 02-covers
rotate 1
wait 200
expect files.per_page is 10
expect files.tiles is pipppppp
shot 02-covers-sideways
rotate 0
wait 200
expect files.per_page is 9
tap library files.item.IMG_0001.jpg
wait 300
expect files.card is picture
shot 03-card
tap library files.card.cancel
wait 200
expect files.card is
tap library files.item.Broken.pdf
wait 300
expect files.card is pdf
tap library files.card.cancel
wait 200
tap library files.view
wait 200
expect files.view is list
expect files.tiles is
expect files.row.1 is 2027 Planner.pdf
shot 04-list
tap library files.view
wait 300
expect files.view is covers
tap library files.crumb.1
wait 300
tap library files.back
wait 300
expect files.open is 0
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 300
expect files.view is covers
waitfor files.tiles is pipppppp
expect files.making is 0
expect files.asked is 8
quit
