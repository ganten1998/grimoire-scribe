# settings: status_mode=1
# White ink and the stroke eraser on a page whose paper is a picture (an imported page: black
# bars across white). White ink restores the picture under it, the bars too, live as a fresh render
# draws it (editor.live_matches_render); the stroke eraser takes nothing where the ink does not
# show: where white ink took it away, and over a bar (the bar is darker than the ink).
# copy: tests/fixtures/scen-import import
# settings: mirror_pdf=0
# settings: marker.size_min_mm=8
# settings: marker.size_max_mm=8
# settings: marker.opacity_min=1
# settings: marker.grain=0
# settings: eraser.size_min_mm=1
# settings: eraser.size_max_mm=1
# The import takes about a second here (waitbg waits for it); its toast shows for 8 s.
# rust-only
waitbg
wait 300
expect library.items is 1
tap 326 608
wait 600
expect screen is editor
pentap panel next
wait 400
pentap panel next
wait 400
expect editor.page is 3
expect editor.paper is 1366081652
# A pencil line down the page, through the bars (at y 744-812, 930-998, 1116-1184 here)
pen line 600 300 600 1300 80 0.7 0.7
wait 300
expect editor.strokes is 1
# White marker along a bar: the bar stays, as it does drawn afresh
pentap panel tool-picker
wait 200
pentap 1425 878
wait 200
pentap panel picker.tool:marker
wait 200
expect settings.white is 1
pen line 300 778 1300 778 40 1.0 1.0
wait 700
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
# And across the line between two bars: the line's ink goes there
pen line 300 870 1300 870 40 1.0 1.0
wait 700
shot 01-white-on-the-picture
expect editor.strokes is 3
expect editor.erase_strokes is 2
expect editor.live_matches_render is 1
# The stroke eraser where white took the line away, then over a bar: the line stays
eraser line 596 850 604 890 8
wait 700
expect editor.strokes is 3
eraser line 596 1140 604 1160 8
wait 700
expect editor.strokes is 3
# Between two bars, where it shows: the line goes
eraser line 596 1040 604 1080 8
wait 700
shot 02-line-taken-where-it-shows
expect editor.strokes is 2
expect editor.erase_strokes is 2
expect editor.live_matches_render is 1
pentap panel library
wait 600
expect screen is library
quit
