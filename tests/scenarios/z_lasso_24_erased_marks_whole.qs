# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# Issue #187, the steps reported, with the Whole strokes eraser (the default) on the page pasted to; the page
# copied from erased with white ink, which is erasing too (a gap in a line, white fountain pen). The stroke
# eraser takes the strokes it touches off the page whole: nothing of them is left behind. The white ink that goes
# with the pasted line stays with it, and the halo of a selection never crosses it (z_lasso_23 has the precise
# eraser). display.dark counts what the panel shows dark in a place: 0 at every erased place.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect settings.eraser_mode is 0
# page 1: a line down the left (it keeps the selection's box away from the places checked), a word and a line
pen line 150 600 150 1650 40 0.6 0.6
pen curve 400 750 40 60 0.6 4
pen line 400 1150 1400 1150 60 0.6 0.6
wait 300
expect editor.strokes is 3
expect display.dark.822.1135.16.30 not 0
# white fountain pen: a gap in the line
tap panel tool-picker
wait 100
tap panel picker.tool:fountain
wait 100
expect editor.tool is fountain
tap panel tool-picker
wait 100
tap panel picker.ink.1
wait 100
tap 100 2400
wait 100
expect settings.white is 1
pen line 812 1110 812 1190 20 1.0 1.0
pen line 820 1110 820 1190 20 1.0 1.0
pen line 828 1110 828 1190 20 1.0 1.0
pen line 836 1110 836 1190 20 1.0 1.0
pen line 844 1110 844 1190 20 1.0 1.0
pen leave
wait 300
tap panel tool-picker
wait 100
tap panel picker.ink.0
wait 100
tap 100 2400
wait 100
expect settings.white is 0
expect editor.strokes is 8
expect editor.erase_strokes is 5
expect editor.live_matches_render is 1
expect display.dark.822.1135.16.30 is 0
# the whole page lassoed: the line's halo stops at its gap
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
expect display.dark.822.1135.16.30 is 0
shot 01-page1-selected
# copied, pasted on the next page where it was: the gap's white ink carried with the line
tap panel select.copy
wait 300
pentap panel next
wait 400
expect editor.page is 2
tap panel select.paste
wait 300
expect editor.strokes is 8
expect editor.erase_strokes is 5
expect editor.selected is 3
expect editor.live_matches_render is 1
tap panel select.done
wait 300
# the stroke eraser through the pasted word: it goes whole; a line added and taken away the same way
eraser line 600 690 600 810 12
wait 300
pen line 400 1450 1100 1450 60 0.6 0.6
pen leave
wait 300
eraser line 700 1420 700 1480 8
wait 2000
expect editor.strokes is 7
expect editor.erase_strokes is 5
expect editor.live_matches_render is 1
expect display.dark.400.690.500.110 is 0
expect display.dark.450.1430.600.40 is 0
expect display.dark.822.1135.16.30 is 0
# a lasso round where the word was: the stroke eraser left nothing of it, nothing to select (as a lasso round ink
# the precise eraser or white ink erased wholly, issue #205: z_lasso_29)
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
pen down 330 680
pen move 950 680
pen move 950 820
pen move 330 820
pen up 330 690
wait 300
expect editor.select is armed
expect editor.selected is 0
expect toast is Nothing to select.
# the whole page lassoed again: the line down the left and the line with its gap, the gap clear
pen down 100 550
pen move 1600 550
pen move 1600 1750
pen move 100 1750
pen up 100 570
wait 300
expect editor.select is selected
expect editor.selected is 2
expect editor.halo is 2
expect display.dark.822.1135.16.30 is 0
shot 02-page2-selected
# the page it was copied from, lassoed again: the gap whited out there is still clear
tap panel select.done
wait 300
pentap panel previous
wait 400
expect editor.page is 1
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
expect display.dark.822.1135.16.30 is 0
expect editor.live_matches_render is 1
quit
