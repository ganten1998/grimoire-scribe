# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# settings: eraser.size_min_mm=2
# settings: eraser.size_max_mm=2
# settings: current_tool=7
# settings: marker.size_min_mm=2.8
# settings: marker.size_max_mm=2.8
# rust-only
# Issue #187: ink erased by several passes of the precise eraser, none of which alone covers the width of the ink.
# The page shows nothing there, and a selection's halo goes as the page shows ink (lasso.rs stroke_shown_runs),
# so none goes there either, while the lasso is drawn or after. Before, it went by the enclosure's estimate, which
# counts ink erased only where one pass covers its whole width: a halo bridged the erased stretch of a wide marker
# line, and dotted a thin line scrubbed out by passes across it (the faint marks again).
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
# a 2.8 mm marker line, its middle erased wholly by 7 passes along it, 6 px apart (a 2 mm eraser)
pen line 400 1150 1400 1150 60 0.6 0.6
pen leave
wait 300
expect display.dark.750.1110.200.80 not 0
eraser line 700 1132 1000 1132 30
eraser line 700 1138 1000 1138 30
eraser line 700 1144 1000 1144 30
eraser line 700 1150 1000 1150 30
eraser line 700 1156 1000 1156 30
eraser line 700 1162 1000 1162 30
eraser line 700 1168 1000 1168 30
wait 300
# a ballpoint line, scrubbed out across its middle by passes 18 px apart
tap panel tool-picker
wait 100
tap panel picker.tool:ballpoint
wait 100
expect editor.tool is ballpoint
pen line 400 1450 1400 1450 60 0.6 0.6
pen leave
wait 300
expect display.dark.750.1430.200.40 not 0
eraser line 700 1400 700 1500 10 0.6 0.6
eraser line 718 1400 718 1500 10 0.6 0.6
eraser line 736 1400 736 1500 10 0.6 0.6
eraser line 754 1400 754 1500 10 0.6 0.6
eraser line 772 1400 772 1500 10 0.6 0.6
eraser line 790 1400 790 1500 10 0.6 0.6
eraser line 808 1400 808 1500 10 0.6 0.6
eraser line 826 1400 826 1500 10 0.6 0.6
eraser line 844 1400 844 1500 10 0.6 0.6
eraser line 862 1400 862 1500 10 0.6 0.6
eraser line 880 1400 880 1500 10 0.6 0.6
eraser line 898 1400 898 1500 10 0.6 0.6
eraser line 916 1400 916 1500 10 0.6 0.6
eraser line 934 1400 934 1500 10 0.6 0.6
eraser line 952 1400 952 1500 10 0.6 0.6
eraser line 970 1400 970 1500 10 0.6 0.6
eraser line 988 1400 988 1500 10 0.6 0.6
wait 300
# (the panel settled where it is checked: the eraser's fast refreshes show the paper's gray rules dark until its
# grays refresh, which a loaded machine sends later than a fixed wait allows)
waitfor display.stale.750.1110.200.80 is 0
waitfor display.stale.750.1430.200.40 is 0
expect editor.strokes is 27
expect editor.live_matches_render is 1
expect display.dark.750.1110.200.80 is 0
expect display.dark.750.1430.200.40 is 0
# a lasso round the page, drawn slowly: the preview's halo takes the marker line in, and nothing over its erased
# middle (between two of the paper's rules: the lasso's fast refreshes show the gray rules dark until the lift)
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 100 550
pen move 1600 550
pen move 1600 1750
wait 200
pen move 100 1750
wait 300
expect editor.select is lasso
expect editor.halo is 3
expect display.dark.750.1108.200.58 is 0
pen up 100 570
wait 300
expect editor.select is selected
expect editor.selected is 3
expect editor.halo is 3
expect display.dark.750.1110.200.80 is 0
expect display.dark.750.1430.200.40 is 0
expect display.dark.0.550.1650.1250 not 0
tap panel select.done
wait 300
expect display.dark.750.1110.200.80 is 0
expect display.dark.750.1430.200.40 is 0
quit
