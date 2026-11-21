# settings: status_mode=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# settings: ballpoint.size_min_mm=8
# settings: ballpoint.size_max_mm=8
# rust-only
# Issue #205: a lasso round ink the page shows nothing of (erased wholly by the precise eraser, or whited out)
# selects nothing and says "Nothing to select.", as a lasso round empty paper does, in whole strokes and in exact
# area, and so does the mode button when the lasso judged again in the other mode takes only such ink. "Shows" is
# the halo's rule (lasso.rs stroke_shown_runs, the page as drawn): such a selection would have had no halo, only
# a box round nothing. Beside ink that shows, erased ink is selected as before, and Delete takes it with the rest.
# A lasso round empty paper answers as before.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.tool is pencil
# three words and a long line down the right
pen curve 400 700 40 60 0.6 4
pen curve 400 900 40 60 0.6 4
pen curve 400 1100 40 60 0.6 4
pen line 1150 600 1150 1500 60 0.6 0.6
wait 300
# the precise eraser: the second word wholly (a third pass over what is gone already is not kept)
eraser line 300 870 960 870 40
eraser line 300 900 960 900 40
eraser line 300 930 960 930 40
# white ink (an 8 mm ballpoint): the third word wholly
tap panel tool-picker
wait 100
tap panel picker.tool:ballpoint
wait 100
tap panel tool-picker
wait 100
tap panel picker.ink.1
wait 100
tap 100 2400
wait 100
expect settings.white is 1
pen line 300 1080 960 1080 40 1.0 1.0
pen line 300 1120 960 1120 40 1.0 1.0
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
waitfor display.stale.300.600.1000.1000 is 0
expect editor.strokes is 8
expect editor.erase_strokes is 4
expect editor.live_matches_render is 1
expect display.dark.360.640.540.120 not 0
expect display.dark.360.850.540.100 is 0
expect display.dark.360.1050.540.100 is 0
# Select mode: a lasso round the word the eraser took away selects nothing
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.sel_exact is 0
pen down 330 840
pen move 960 840
pen move 960 965
pen move 330 965
pen up 330 850
wait 300
expect editor.select is armed
expect editor.selected is 0
expect toast is Nothing to select.
expect display.dark.360.850.540.100 is 0
# round the word white ink covers: the same
pen down 650 1165
pen move 960 1165
pen move 960 1040
pen move 330 1040
pen move 330 1165
pen up 600 1165
wait 300
expect editor.select is armed
expect editor.selected is 0
expect toast is Nothing to select.
# round empty paper: as before (round white ink alone: z_lasso_1)
pen down 1300 2000
pen move 1600 2000
pen move 1600 2150
pen move 1300 2150
pen up 1300 2010
wait 300
expect editor.select is armed
expect toast is Nothing to select.
# round the first word and the erased one: both selected (the erased one has no halo), and Delete takes both
pen down 330 640
pen move 960 640
pen move 960 965
pen move 330 965
pen up 330 650
wait 300
expect editor.select is selected
expect editor.selected is 2
expect editor.halo is 1
tap panel select.delete
wait 300
expect editor.strokes is 6
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
pentap panel undo
wait 400
expect editor.strokes is 8
expect editor.live_matches_render is 1
# exact area: a lasso round the erased word alone selects nothing either
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
tap panel select.mode
wait 100
expect editor.sel_exact is 1
# (the last toast gone first, so the one checked is this lasso's)
waitfor toast is
pen down 330 840
pen move 960 840
pen move 960 965
pen move 330 965
pen up 330 850
wait 300
expect editor.select is armed
expect editor.selected is 0
expect toast is Nothing to select.
# round the erased word and a short piece of the line: the piece shows, both are selected; judged again in whole
# strokes (the mode button), the line is not (most of it is outside) and the erased word alone is nothing to
# select: the selection goes, Select mode stays
waitfor toast is
pen down 330 840
pen move 1180 840
pen move 1180 965
pen move 330 965
pen up 330 850
wait 300
expect editor.select is selected
expect editor.selected is 2
expect editor.sel_cut is 1
expect editor.halo is 1
tap panel select.mode
wait 300
expect editor.sel_exact is 0
expect editor.select is armed
expect editor.selected is 0
expect toast is Nothing to select.
expect editor.strokes is 8
expect editor.live_matches_render is 1
quit
