# settings: status_mode=1
# env: GRIMOIRE_BG_DELAY_MS=3000
# A PDF in grimoire/templates (2.2, #172): the editor's Paper sheet > More layouts... shows one waiting tile for it
# until its job has counted the pages (the job is held 3 s here, so the tile is seen), then the folder is read again
# and the PDF's pages take its place, a tile a page named after the file and the page, each filled in once its picture
# is made. A page tapped becomes the open page's paper as a picture template does: its picture is made for the notebook
# in the background and drawn, and the sheet then selects that page. The PDF never becomes a notebook.
# copy: tests/fixtures/pdf/planner templates
# rust-only
wait 300
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 5
pentap panel paper
wait 200
pentap panel paper.more
wait 300
expect editor.modal is templates
expect templates.count is 1
expect templates.ready is 0
expect templates.names is 2027 Planner
shot 01-one-tile-waits
waitbg
wait 300
expect templates.count is 7
expect templates.ready is 7
expect templates.names is 2027 Planner p. 1|2027 Planner p. 2|2027 Planner p. 3|2027 Planner p. 4|2027 Planner p. 5|2027 Planner p. 6|2027 Planner p. 7
shot 02-a-tile-a-page
# page 3, the third tile
pentap 790 452
wait 300
expect editor.modal is none
waitbg
wait 300
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 03-on-page-3
pentap panel paper
wait 200
pentap panel paper.more
wait 300
expect templates.selected is 2027 Planner p. 3
expect templates.count is 7
pentap panel templates.back
wait 200
expect editor.modal is paper
tap 1562 2125
wait 200
expect editor.modal is none
pentap panel library
wait 600
expect screen is library
expect library.items is 1
quit
