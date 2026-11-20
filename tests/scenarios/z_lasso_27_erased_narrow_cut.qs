# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# settings: eraser.size_min_mm=2
# settings: eraser.size_max_mm=2
# settings: current_tool=7
# settings: marker.size_min_mm=2.8
# settings: marker.size_max_mm=2.8
# rust-only
# Issue #187 with one pass of the precise eraser (2 mm) across ink wider than it: a 2.8 mm marker line and a
# highlighter line (5.6 mm), each cut across. The page shows nothing in either cut, and neither does a selection's
# halo: it goes as the page shows the stroke's ink across it (lasso.rs stroke_shown_runs). Before, each place in
# the cut had the ink either side of it within the ink's radius, and the halo went across the cut.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.tool is marker
# a line down the left (it keeps the selection's box away from the places checked)
pen line 150 600 150 1650 40 0.6 0.6
pen line 400 1150 1400 1150 60 0.6 0.6
pen leave
wait 300
expect display.dark.826.1100.8.100 not 0
eraser line 830 1050 830 1250 20
tap panel tool-picker
wait 100
tap panel picker.tool:highlighter
wait 100
expect editor.tool is highlighter
pen line 400 1450 1400 1450 60 0.6 0.6
pen leave
wait 300
# (the highlighter's gray is lighter than what display.dark counts; a halo's dither is not)
expect editor.strokes is 4
eraser line 830 1350 830 1550 20
# (the panel settled where it is checked: the eraser's fast refreshes show the paper's gray rules dark until its
# grays refresh)
waitfor display.stale.826.1100.8.100 is 0
waitfor display.stale.826.1400.8.100 is 0
expect editor.strokes is 5
expect editor.live_matches_render is 1
expect display.dark.826.1100.8.100 is 0
expect display.dark.826.1400.8.100 is 0
# the whole page lassoed: both lines have a halo either side of their cut and none in it
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 100 550
pen move 1600 550
pen move 1600 1750
pen move 100 1750
pen up 100 570
wait 300
expect editor.select is selected
expect editor.selected is 3
expect editor.halo is 3
expect display.dark.826.1100.8.100 is 0
expect display.dark.826.1400.8.100 is 0
expect display.dark.0.550.1650.1250 not 0
tap panel select.done
wait 300
expect display.dark.826.1100.8.100 is 0
expect display.dark.826.1400.8.100 is 0
quit
