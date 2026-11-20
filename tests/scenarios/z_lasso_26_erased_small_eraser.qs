# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# settings: eraser.size_min_mm=1
# settings: eraser.size_max_mm=1
# settings: current_tool=2
# rust-only
# Issue #187 with the precise eraser at 1 mm, narrower than its passes are apart or than the ink: a thin line
# scrubbed out by passes 9 px apart across it, and another with a 1 mm gap. The page shows nothing at either, and
# neither does a selection's halo (it goes as the page shows ink, lasso.rs stroke_shown_runs). Exact area: the
# preview's halo goes along the samples a crossing stroke is judged by that the page shows, never joined across
# the gap (they skip it, and were joined when 1.6 mm or less apart); nor is the piece's after the lift.
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.tool is ballpoint
pen line 150 600 150 1650 40 0.6 0.6
pen line 400 1150 1400 1150 60 0.6 0.6
pen line 400 1450 1400 1450 60 0.6 0.6
pen leave
wait 300
expect display.dark.750.1130.200.40 not 0
eraser line 700 1100 700 1200 10 0.6 0.6
eraser line 709 1100 709 1200 10 0.6 0.6
eraser line 718 1100 718 1200 10 0.6 0.6
eraser line 727 1100 727 1200 10 0.6 0.6
eraser line 736 1100 736 1200 10 0.6 0.6
eraser line 745 1100 745 1200 10 0.6 0.6
eraser line 754 1100 754 1200 10 0.6 0.6
eraser line 763 1100 763 1200 10 0.6 0.6
eraser line 772 1100 772 1200 10 0.6 0.6
eraser line 781 1100 781 1200 10 0.6 0.6
eraser line 790 1100 790 1200 10 0.6 0.6
eraser line 799 1100 799 1200 10 0.6 0.6
eraser line 808 1100 808 1200 10 0.6 0.6
eraser line 817 1100 817 1200 10 0.6 0.6
eraser line 826 1100 826 1200 10 0.6 0.6
eraser line 835 1100 835 1200 10 0.6 0.6
eraser line 844 1100 844 1200 10 0.6 0.6
eraser line 853 1100 853 1200 10 0.6 0.6
eraser line 862 1100 862 1200 10 0.6 0.6
eraser line 871 1100 871 1200 10 0.6 0.6
eraser line 880 1100 880 1200 10 0.6 0.6
eraser line 889 1100 889 1200 10 0.6 0.6
eraser line 898 1100 898 1200 10 0.6 0.6
eraser line 907 1100 907 1200 10 0.6 0.6
eraser line 916 1100 916 1200 10 0.6 0.6
eraser line 925 1100 925 1200 10 0.6 0.6
eraser line 934 1100 934 1200 10 0.6 0.6
eraser line 943 1100 943 1200 10 0.6 0.6
eraser line 952 1100 952 1200 10 0.6 0.6
eraser line 961 1100 961 1200 10 0.6 0.6
eraser line 970 1100 970 1200 10 0.6 0.6
eraser line 979 1100 979 1200 10 0.6 0.6
eraser line 988 1100 988 1200 10 0.6 0.6
eraser line 997 1100 997 1200 10 0.6 0.6
eraser line 830 1390 830 1510 12
wait 300
expect editor.strokes is 38
expect editor.live_matches_render is 1
expect display.dark.750.1130.200.40 is 0
expect display.dark.828.1440.4.20 is 0
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
expect display.dark.750.1130.200.40 is 0
expect display.dark.828.1440.4.20 is 0
expect display.dark.0.550.1650.1250 not 0
tap panel select.done
wait 300
# exact area across the gap (between two of the paper's rules, which the preview's fast refreshes show dark)
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
tap panel select.mode
wait 100
pen down 600 1350
pen move 1100 1350
pen move 1100 1550
wait 200
pen move 600 1550
wait 300
expect editor.select is lasso
expect editor.halo is 1
expect display.dark.828.1440.4.20 is 0
pen up 600 1360
wait 300
expect editor.select is selected
expect editor.sel_cut is 1
expect editor.halo is 1
# (inside the selection's box, whose edges run along the line just above and below)
expect display.dark.828.1445.4.11 is 0
tap panel select.mode
wait 100
tap panel select.done
wait 300
quit
