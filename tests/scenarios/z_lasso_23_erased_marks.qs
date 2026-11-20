# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# rust-only
# Issue #187, the steps reported, with the precise eraser: a page lassoed whole, copied, pasted on the next page,
# some of the pasted ink erased, a few bits added and erased, the page lassoed whole again; then the page it was
# copied from lassoed again. Erased ink stays in the page file under its erase strokes (docs/FORMAT.md), and the
# lasso selects it with what shows of its stroke, or by its whole spine when nothing shows (LASSO-DESIGN.md 6.2);
# its halo (PIXEL-LASSO.md 2) goes only along what shows. Before 2.2 it went along every point of every selected
# stroke, so a lasso showed a dithered ghost of each erased word and bridged each erased gap ("faint marks").
# display.dark counts what the panel shows dark in a place: 0 at every erased place, with the selection and
# without it; the page drawn live equals a fresh drawing after every step.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.scale is 1.00
# page 1: a line down the left (it keeps the selection's box away from the places checked), two words and a line
pen line 150 600 150 1650 40 0.6 0.6
pen curve 400 750 40 60 0.6 4
pen curve 400 950 40 60 0.6 4
pen line 400 1150 1400 1150 60 0.6 0.6
wait 300
expect editor.strokes is 4
expect display.dark.420.900.440.100 not 0
expect display.dark.815.1130.30.40 not 0
# the precise eraser: the second word wholly (two passes; a third over what is gone already changes nothing and is
# not kept), and a gap in the line
eraser line 300 920 960 920 40
eraser line 300 950 960 950 40
eraser line 300 980 960 980 40
eraser line 830 1090 830 1210 12
wait 300
expect editor.strokes is 7
expect editor.erase_strokes is 3
expect editor.live_matches_render is 1
expect display.dark.420.900.440.100 is 0
expect display.dark.815.1130.30.40 is 0
# a lasso drawn round the erased word alone: the preview shows no halo there and refreshes nothing for it
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 330 870
pen move 950 870
pen move 950 1030
wait 200
pen move 330 1030
wait 200
expect editor.select is lasso
expect editor.preview_batches is 2
expect editor.halo is 0
expect editor.preview_refreshes is 0
# (between two of the paper's rules: the lasso's fast refreshes show the gray rules dark until the lift)
expect display.dark.420.912.440.50 is 0
pen up 330 880
wait 300
expect editor.select is selected
expect editor.selected is 1
expect editor.halo is 0
expect display.dark.420.910.440.80 is 0
# the whole page lassoed (the pen down outside the selection ends it): the erased word is selected with the
# rest (nothing of it shows: judged by its whole spine), but has no halo; the line's halo stops at its gap
pen down 100 550
pen move 1600 550
pen move 1600 1750
pen move 100 1750
pen up 100 570
wait 300
expect editor.select is selected
expect editor.selected is 4
expect editor.halo is 3
expect display.dark.420.900.440.100 is 0
expect display.dark.815.1130.30.40 is 0
expect display.dark.0.550.1650.1250 not 0
shot 01-page1-selected
# Copy leaves the erased word out (erased content does not travel); the line's gap goes with it (a copy of its
# erase stroke); pasted on the next page where it was
tap panel select.copy
wait 300
pentap panel next
wait 400
expect editor.page is 2
tap panel select.paste
wait 300
expect editor.strokes is 4
expect editor.erase_strokes is 1
expect editor.layered is 0
expect editor.selected is 3
expect editor.halo is 3
expect editor.live_matches_render is 1
expect display.dark.815.1130.30.40 is 0
tap panel select.done
wait 300
# some of the pasted ink erased (a strip through the word, another gap in the line), a line added and erased
pentap panel eraser
wait 100
pen line 600 690 600 810 12 0.6 0.6
pen line 600 1090 600 1210 12 0.6 0.6
wait 300
pentap panel eraser
wait 100
pen line 400 1450 1100 1450 60 0.6 0.6
pen leave
wait 300
pentap panel eraser
wait 100
pen line 380 1450 1120 1450 60 0.6 0.6
wait 300
pentap panel eraser
wait 2000
expect editor.strokes is 8
expect editor.erase_strokes is 4
expect editor.live_matches_render is 1
expect display.dark.590.700.20.100 is 0
expect display.dark.590.1130.20.40 is 0
expect display.dark.815.1130.30.40 is 0
expect display.dark.450.1430.600.40 is 0
# the whole page lassoed again: the erased line is selected (nothing of it shows) with no halo; no halo crosses
# the strip or either gap
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
expect editor.selected is 4
expect editor.halo is 3
expect display.dark.590.700.20.100 is 0
expect display.dark.590.1130.20.40 is 0
expect display.dark.815.1130.30.40 is 0
expect display.dark.450.1430.600.40 is 0
expect editor.live_matches_render is 1
shot 02-page2-selected
# the page it was copied from, lassoed again: the word erased there before the copy does not come back
tap panel select.done
wait 300
pentap panel previous
wait 400
expect editor.page is 1
expect editor.strokes is 7
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
expect editor.selected is 4
expect editor.halo is 3
expect display.dark.420.900.440.100 is 0
expect display.dark.815.1130.30.40 is 0
tap panel select.done
wait 300
expect display.dark.420.900.440.100 is 0
expect editor.live_matches_render is 1
# the erased word alone selected and moved: what shows of it (nothing) is found once at its new place and kept,
# not found again at every redraw while the selection lasts (a page of such ink took tens of ms a redraw)
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 330 870
pen move 950 870
pen move 950 1030
wait 200
pen move 330 1030
wait 200
pen up 330 880
wait 300
expect editor.select is selected
expect editor.selected is 1
expect editor.preview_reused is 1
expect editor.halo_found is 0
expect editor.sel_box is 360 895 536 111
pen down 640 950
pen move 640 990
wait 150
pen move 640 1030
wait 150
pen up 640 1030
wait 600
expect editor.select is selected
expect editor.undo_kind is lasso
expect editor.sel_box is 360 975 536 111
expect editor.halo is 0
expect editor.halo_found is 1
# (the tool picker over it, and gone: the selection drawn again)
tap panel tool-picker
wait 300
tap panel tool-picker
wait 300
expect editor.select is selected
expect editor.halo_found is 1
expect display.dark.420.990.440.80 is 0
expect editor.live_matches_render is 1
quit
