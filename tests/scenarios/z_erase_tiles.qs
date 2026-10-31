# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=0
# rust-only
# The stroke eraser draws the page again only in the tiles the strokes it takes off drew into
# (page.rs page_render_tiles), not their whole boxes: a long stroke across a page of rows taken off
# alone, then a sweep down through three rows, then undo and redo. After each the page is exactly a
# fresh drawing of its strokes (live_matches_render), and editor.erase_tiles shows the tiles drawn
# again against the tiles of the boxes (all it drew before 2.0.0-rc.12).
wait 300
tap 965 95
wait 100
tap library paper.0
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen line 600 500 1400 500 40 0.6 0.6
pen line 600 700 1400 700 40 0.6 0.6
pen line 600 900 1400 900 40 0.6 0.6
pen line 600 1100 1400 1100 40 0.6 0.6
pen line 600 1300 1400 1300 40 0.6 0.6
pen line 600 1500 1400 1500 40 0.6 0.6
pen line 600 1700 1400 1700 40 0.6 0.6
pen line 600 1900 1400 1900 40 0.6 0.6
pen line 300 400 1700 2300 120 0.8 0.8
wait 700
expect editor.strokes is 9
expect editor.erase_tiles is 0 0
# The long stroke alone, where it passes between the first rows
eraser line 570 800 620 800 6
wait 700
shot 01-long-stroke-gone
expect editor.strokes is 8
expect editor.live_matches_render is 1
expect editor.erase_tiles is 391 11011
# Down through the first three rows
eraser line 1000 450 1000 1000 20
wait 700
shot 02-three-rows-gone
expect editor.strokes is 5
expect editor.live_matches_render is 1
expect editor.erase_tiles is 544 11488
pentap panel undo
wait 300
expect editor.strokes is 8
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.strokes is 5
expect editor.live_matches_render is 1
pentap panel library
wait 500
expect screen is library
quit
