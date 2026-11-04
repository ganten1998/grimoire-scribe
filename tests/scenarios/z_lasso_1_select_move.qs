# settings: status_mode=1
# settings: mirror_pdf=0
# rust-only
# Lasso select and move (docs/beta2/LASSO-DESIGN.md): Select from the tool picker, a lasso around a word
# selects it (not its neighbour), a drag moves it as one undo step, undo and redo swap it back exactly; a
# word partly erased with white ink moves with its erasing (the neighbour the same white stroke crossed stays
# erased); copy and paste on the same page lands 3 mm off over the original ink, scoped so it never
# lightens it; cut and delete; a lasso around white ink alone selects nothing. The page drawn live equals a
# fresh drawing after every step.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
# a ballpoint word and a neighbour
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.tool is ballpoint
pen curve 450 700 40 60 0.6 3
pen line 350 820 750 820 40 0.6
pen line 1100 700 1400 820 40 0.6
wait 100
expect editor.strokes is 3
# Select mode from the tool picker
tap panel tool-picker
wait 100
expect editor.modal is tools
tap panel picker.select
wait 100
expect editor.modal is none
expect editor.select is armed
expect editor.strokes is 3
# a lasso around the word
pen down 280 580
pen move 500 570
pen move 820 580
pen move 830 750
pen move 820 900
pen move 500 910
pen move 280 900
pen move 270 750
pen up 275 600
wait 100
expect editor.select is selected
expect editor.selected is 2
expect editor.strokes is 3
expect editor.live_matches_render is 1
shot 01-selected
# drag it: only the outline moves until the release
pen down 500 760
pen move 600 900
pen move 700 1100
pen move 800 1300
expect editor.select is dragging
expect editor.undo_kind is add
pen up 800 1300
wait 100
expect editor.select is selected
expect editor.strokes is 3
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
expect editor.layered is 0
shot 02-moved
# undo and redo: one step, exact
tap panel select.done
wait 100
expect editor.select is off
pentap panel undo
wait 100
expect editor.strokes is 3
expect editor.redo_kind is lasso
expect editor.live_matches_render is 1
pentap panel redo
wait 100
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
pentap panel undo
wait 100
# white ink through the word and the neighbour
tap panel tool-picker
tap panel picker.ink.1
wait 100
expect settings.white is 1
tap 100 2400
wait 100
expect editor.modal is none
pen line 300 700 1450 760 60 0.8
wait 100
expect editor.strokes is 4
tap panel tool-picker
tap panel picker.ink.0
wait 100
tap 100 2400
wait 100
expect settings.white is 0
expect editor.modal is none
# select the word again and move it: its erasing goes with it, the neighbour stays erased
tap panel tool-picker
tap panel picker.select
wait 100
pen down 280 580
pen move 500 570
pen move 820 580
pen move 830 750
pen move 820 900
pen move 500 910
pen move 280 900
pen move 270 750
pen up 275 600
wait 100
expect editor.selected is 2
pen down 500 760
pen move 600 1000
pen move 600 1500
pen up 600 1500
wait 100
expect editor.undo_kind is lasso
expect editor.strokes is 5
expect editor.layered is 0
expect editor.live_matches_render is 1
shot 03-moved-erased
# copy and paste on the same page: 3 mm off, over the original ink: scoped, never lightening it
tap panel select.copy
wait 100
expect editor.clipboard is 1
expect toast is
expect editor.undo_kind is lasso
tap panel select.paste
wait 100
expect editor.select is selected
expect editor.selected is 2
expect editor.strokes is 9
expect editor.layered is 1
expect editor.live_matches_render is 1
shot 04-pasted
# undo and redo of the scoped paste refresh only the group's place (its floor marker, off the page,
# counts for nothing): never the whole screen. First past the launch's second repaint (3.5 s after the
# start, a whole-screen ui of its own), which a run reaching here around then caught in the window.
wait 1500
expect display.presents has
pentap panel undo
wait 100
expect editor.strokes is 5
expect editor.live_matches_render is 1
expect display.presents is fast 1718 760 100 100, ui 10 220 1822 2220
pentap panel redo
wait 100
expect editor.strokes is 9
expect editor.live_matches_render is 1
expect display.presents is fast 1718 868 100 100, ui 428 220 1404 1734
# cut the pasted group: off the page, one step
pen down 500 1300
pen move 1000 1300
pen move 1000 1800
pen move 500 1800
pen up 500 1350
wait 100
expect editor.select is selected
tap panel select.cut
wait 100
expect editor.select is armed
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
# a lasso around white ink alone selects nothing
pen down 1420 700
pen move 1520 700
pen move 1520 820
pen move 1420 820
pen up 1420 710
wait 100
expect editor.select is armed
expect toast is Nothing to select: white ink and eraser marks move with the ink under them.
# choosing a pen leaves Select mode
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.select is off
expect editor.live_matches_render is 1
quit
