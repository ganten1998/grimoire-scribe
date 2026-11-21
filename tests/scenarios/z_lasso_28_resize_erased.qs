# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# rust-only
# Issue #204: a selection made larger or smaller by its corner keeps its erasing in scale. A resize scales each
# stroke's widths with its points, black ink, white ink and the erasing that goes with it alike (lasso.rs
# lasso_scale_widths), so the page after Done looks as the selection did, larger or smaller. Before, the widths
# stayed: made larger, the erasing no longer covered the ink it had erased and the page showed traces of an erased
# word (at 1.1 times faint ones, which the halo outlined; at 1.5 dark ones) and of a line cut by white ink; made
# smaller, the erasing took away ink that showed beside a cut. Undo puts the page back exactly; erasing made larger
# over ink outside the selection is scoped to its floor and leaves that ink as it was.
# display.dark counts what the panel shows dark in a place; editor.halo the selected strokes the page shows some of.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
# two words, a marker line, a line, and a line down the right that the lasso leaves out
expect editor.tool is pencil
pen curve 400 700 40 60 0.6 4
pen curve 400 900 40 60 0.6 4
tap panel tool-picker
wait 100
tap panel picker.tool:marker
wait 100
expect editor.tool is marker
pen line 400 1100 960 1100 60 0.6 0.6
tap panel tool-picker
wait 100
tap panel picker.tool:pencil
wait 100
expect editor.tool is pencil
pen line 400 1250 960 1250 60 0.6 0.6
pen line 1150 900 1150 1300 40 0.6 0.6
wait 300
# the precise eraser: the second word wholly (a third pass over what is gone already is not kept), a cut in a line
eraser line 300 870 960 870 40
eraser line 300 900 960 900 40
eraser line 300 930 960 930 40
eraser line 680 1040 680 1160 12
# white fountain pen: a gap in the other line
tap panel tool-picker
wait 100
tap panel picker.tool:fountain
wait 100
tap panel tool-picker
wait 100
tap panel picker.ink.1
wait 100
tap 100 2400
wait 100
expect settings.white is 1
pen line 662 1210 662 1290 20 1.0 1.0
pen line 670 1210 670 1290 20 1.0 1.0
pen line 678 1210 678 1290 20 1.0 1.0
pen line 686 1210 686 1290 20 1.0 1.0
pen line 694 1210 694 1290 20 1.0 1.0
pen leave
wait 300
tap panel tool-picker
wait 100
tap panel picker.ink.0
wait 100
tap 100 2400
wait 100
expect settings.white is 0
# (the panel settled where it is checked: the eraser's fast refreshes show the paper's gray rules dark until its
# grays refresh)
waitfor display.stale.300.600.1000.800 is 0
expect editor.strokes is 13
expect editor.erase_strokes is 8
expect editor.live_matches_render is 1
expect display.dark.360.850.540.100 is 0
expect display.dark.660.1092.40.16 is 0
expect display.dark.666.1242.24.16 is 0
expect display.dark.1140.960.20.120 not 0
# the lasso round the words and the two lines: the erased word is selected (nothing of it shows) with no halo
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1000 600
pen move 1000 1330
pen move 280 1330
pen up 280 620
wait 300
expect editor.select is selected
expect editor.selected is 4
expect editor.halo is 3
expect editor.sel_box is 360 645 616 621
expect editor.page_sig_keep is kept
# made 1.1 times as large by the bottom-right corner: the erased word still shows nothing (no halo, no trace)
pen down 976 1266
pen move 992 1282
wait 150
pen move 1007 1297
wait 150
pen move 1023 1313
wait 150
pen move 1038 1328
wait 150
pen up 1038 1328
wait 600
expect editor.select is selected
expect editor.undo_kind is lasso
expect editor.sel_box is 360 645 678 683
expect editor.live_matches_render is 1
expect editor.halo is 3
expect editor.layered is 0
expect display.dark.360.870.594.111 is 0
expect display.dark.690.1137.44.17 is 0
expect display.dark.697.1302.26.17 is 0
tap panel select.done
wait 300
waitfor display.stale.300.600.1200.1000 is 0
expect display.dark.360.870.594.111 is 0
expect display.dark.690.1137.44.17 is 0
expect display.dark.697.1302.26.17 is 0
# undo puts the page back exactly
pentap panel undo
wait 400
expect editor.strokes is 13
expect editor.page_sig_same is 1
expect editor.live_matches_render is 1
waitfor display.stale.300.600.1200.1000 is 0
expect display.dark.360.850.540.100 is 0
# 1.5 times as large: the white ink's gap stays clear too; the erasing now reaches the line down the right, which
# the lasso left out: its floor is scoped (a layer), and that line shows as it did
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1000 600
pen move 1000 1330
pen move 280 1330
pen up 280 620
wait 300
expect editor.selected is 4
expect editor.sel_box is 360 645 616 621
pen down 976 1266
pen move 1053 1344
wait 150
pen move 1130 1421
wait 150
pen move 1207 1499
wait 150
pen move 1284 1577
wait 150
pen up 1284 1577
wait 600
expect editor.select is selected
expect editor.sel_box is 360 645 924 932
expect editor.live_matches_render is 1
expect editor.halo is 3
expect editor.layered is 1
expect display.dark.360.953.780.150 is 0
expect display.dark.810.1316.60.24 is 0
expect display.dark.819.1541.36.24 is 0
expect display.dark.1140.960.20.120 not 0
shot 01-larger
# made smaller again (to 1.2 times the first size): a group whose erasing is scoped resized, the same
pen down 1284 1576
pen move 1240 1531
wait 150
pen move 1196 1485
wait 150
pen move 1151 1440
wait 150
pen move 1107 1394
wait 150
pen up 1107 1394
wait 600
expect editor.select is selected
expect editor.sel_box is 360 645 745 751
expect editor.live_matches_render is 1
expect editor.halo is 3
expect display.dark.360.891.648.125 is 0
expect display.dark.1140.960.20.120 not 0
tap panel select.done
wait 300
waitfor display.stale.300.600.1500.1300 is 0
expect display.dark.360.891.648.125 is 0
expect display.dark.1140.960.20.120 not 0
# two undos: back exactly as it was
pentap panel undo
wait 400
pentap panel undo
wait 400
expect editor.strokes is 13
expect editor.page_sig_same is 1
expect editor.layered is 0
expect editor.live_matches_render is 1
# half as large: the cut in the line is half as wide, so the line's ink beside it, which showed, still shows (the
# erasing kept at its width took it away); the erased word and the white ink's gap stay clear
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 280 600
pen move 1000 600
pen move 1000 1330
pen move 280 1330
pen up 280 620
wait 300
expect editor.selected is 4
pen down 976 1266
pen move 899 1189
wait 150
pen move 822 1111
wait 150
pen move 745 1034
wait 150
pen move 668 956
wait 150
pen up 668 956
wait 600
expect editor.select is selected
expect editor.sel_box is 360 645 308 311
expect editor.live_matches_render is 1
expect editor.halo is 3
tap panel select.done
wait 300
waitfor display.stale.300.600.1000.800 is 0
expect display.dark.479.866.18.12 not 0
expect display.dark.543.866.18.12 not 0
expect display.dark.360.748.270.50 is 0
expect display.dark.510.868.20.9 is 0
expect display.dark.513.944.12.8 is 0
shot 02-smaller
pentap panel undo
wait 400
expect editor.page_sig_same is 1
expect editor.live_matches_render is 1
quit
