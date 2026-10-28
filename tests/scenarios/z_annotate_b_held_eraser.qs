# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/annotate/sideways import/.annotate
# settings: eraser_mode=1
# settings: current_tool=3
# settings: side_button=2
# pdfs: 1
# z_annotate_a_held_white with the precise eraser (the eraser tool, eraser_mode=1) over highlighter
# ink (the pen's button held): the eraser down while the page's picture is fitted, held across the
# end of that job, then moved on. The page is drawn again with its picture once the pen lifts.
# rust-only
wait 300
expect screen is editor
expect annotate.luma is 255
side on
pen line 300 1000 1000 1000 30 0.8 0.8
side off
pen down 300 1000 0.6
pen move 320 1000 0.6
waitbg
wait 300
expect editor.live_matches_render is 0
pen move 600 1000 0.6
pen move 1000 1000 0.6
pen up 1000 1000
wait 700
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.stroke_look is round tip
expect editor.live_matches_render is 1
expect annotate.luma not 255
pentap panel library
wait 600
expect screen is library
quit
