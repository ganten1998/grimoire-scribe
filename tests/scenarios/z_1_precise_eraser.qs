# settings: status_mode=1
# The precise eraser (eraser_mode=1, set by hand until Pen feel has its switch): the pen's eraser
# end, the eraser button and the pen button (side_button=1) draw erase strokes that take away only
# what they touch and keep the paper's lines, each with the round tip and the eraser's settings
# (editor.stroke_look, editor.stroke_settings). After each step the page drawn afresh is what live
# drawing left (editor.live_matches_render): erasing, undo and redo, Clear page and its undo, a
# paper change and its undo, another page on another paper. An erase stroke that changes nothing
# is not kept. Closing the notebook compacts it in the background: its page file loses the ink
# erased entirely (and the erase stroke then over nothing) and looks just the same.
# settings: eraser_mode=1
# settings: side_button=1
# settings: mirror_pdf=0
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 1
expect settings.eraser_mode is 1
expect editor.paper_cache is 0
# Two lines of handwriting, a ballpoint underline and a pencil line
pen curve 300 700 40 80 0.6 4
pen curve 300 840 40 80 0.6 4
pen line 300 1000 1300 1000 80 0.7 0.7
pen line 300 1200 1300 1250 80 0.5 0.8
wait 300
expect editor.strokes is 4
expect editor.erase_strokes is 0
expect editor.stroke_look is pencil
expect editor.stroke_settings is pencil
# The eraser end through the first line of writing: an erase stroke, kept like ink
eraser line 500 640 560 780 12
wait 700
shot 01-eraser-end
expect editor.strokes is 5
expect editor.erase_strokes is 1
expect editor.stroke_look is round tip
expect editor.stroke_settings is eraser
expect editor.paper_cache is 1
expect editor.live_matches_render is 1
# The eraser button: the pen's tip erases
pentap panel eraser
wait 100
expect editor.tool is eraser
pen line 400 780 700 900 20 0.6 0.6
wait 300
expect editor.strokes is 6
expect editor.erase_strokes is 2
expect editor.stroke_look is round tip
expect editor.stroke_settings is eraser
expect editor.live_matches_render is 1
# Tapped again: back to the pencil. The pen button (side_button=1) erases whatever the tip draws
pentap panel eraser
wait 100
expect editor.tool is pencil
side on
pen line 900 950 1000 1060 15 0.6 0.6
side off
wait 300
expect editor.strokes is 7
expect editor.erase_strokes is 3
expect editor.stroke_look is round tip
expect editor.stroke_settings is eraser
expect editor.live_matches_render is 1
# Over blank paper (its lines are the paper) it changes nothing: not kept, no undo step
eraser line 300 1800 600 1900 15
wait 300
expect editor.strokes is 7
expect editor.erase_strokes is 3
# Undo and redo an erase stroke
pentap panel undo
wait 700
shot 02-undo-erase
expect editor.strokes is 6
expect editor.erase_strokes is 2
expect editor.live_matches_render is 1
pentap panel redo
wait 700
shot 03-redo-erase
expect editor.strokes is 7
expect editor.live_matches_render is 1
# Clear page and its undo
pentap panel clear
wait 300
expect editor.strokes is 0
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.strokes is 7
expect editor.live_matches_render is 1
# Another paper (grand staff): the next erase stroke restores its lines
pentap panel paper
wait 200
expect editor.modal is paper
pentap 1348 1035
wait 400
expect editor.paper is 8
expect editor.live_matches_render is 1
eraser line 300 1180 700 1280 20
wait 700
shot 04-erased-on-grand-staff
expect editor.strokes is 8
expect editor.live_matches_render is 1
# Both undone (the erase stroke, then the paper): medium rule again, and the next erase stroke
# restores medium rule, not the staves it restored last
pentap panel undo
wait 200
pentap panel undo
wait 300
expect editor.paper is 1
expect editor.strokes is 7
expect editor.live_matches_render is 1
eraser line 1100 1150 1150 1300 15
wait 700
shot 05-erased-on-medium-again
expect editor.strokes is 8
expect editor.live_matches_render is 1
# A new page on dot grid, written and erased
pentap panel next
wait 400
expect editor.page is 2
pentap panel paper
wait 200
pentap 511 1035
wait 400
expect editor.paper is 10
pen line 300 700 1300 760 60 0.7 0.7
eraser line 700 650 760 820 10
wait 700
shot 06-page-2-dots
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
# Back on page 1 (medium rule): the paper follows the page
pentap panel previous
wait 400
expect editor.page is 1
expect editor.paper is 1
eraser line 1250 1180 1290 1300 10
wait 300
expect editor.strokes is 9
expect editor.live_matches_render is 1
# A pencil line erased entirely by one pass: it stays in the page file until compaction
pen line 300 1600 700 1600 40 0.6 0.6
eraser line 250 1600 750 1600 40
wait 700
shot 07-before-compaction
expect editor.strokes is 11
expect editor.erase_strokes is 6
expect editor.live_matches_render is 1
# The Library: the notebook closes and is compacted in the background
pentap panel library
wait 300
expect screen is library
wait 2000
expect bg.busy is 0
# Opened again: the page file holds two strokes fewer (the line and the pass over it), and the
# page shows just as it did when it closed
tap 326 608
wait 500
expect screen is editor
expect editor.page is 1
expect editor.strokes is 9
expect editor.erase_strokes is 5
expect editor.matches_closed is 1
expect editor.live_matches_render is 1
shot 08-after-compaction
pentap panel library
wait 500
expect screen is library
expect bg.busy is 0
quit
