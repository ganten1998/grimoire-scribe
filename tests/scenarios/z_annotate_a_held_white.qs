# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/annotate/sideways import/.annotate
# settings: side_button=3
# pdfs: 1
# The page of z_annotate_8_sideways, white while its picture is fitted in the background. Ink on it,
# then white ink (an erase stroke: the pen's button held) over the ink, held down across the end of
# that job: the paper the stroke restores toward is read all along, so it stays as it was until the
# pen lifts, and the page waits too (the editor's page is not the page drawn afresh until then).
# Then the page is drawn again with its picture. White ink over ink on the book's text afterwards
# restores toward the picture: the text stays, and the page the editor holds is the page drawn
# afresh.
# rust-only
wait 300
expect screen is editor
expect annotate.result is made
expect annotate.luma is 255
pen line 300 1000 1000 1000 30 0.8 0.8
side on
pen down 300 1000 0.8
pen move 320 1000 0.8
waitbg
wait 300
expect editor.live_matches_render is 0
pen move 600 1000 0.8
pen move 1000 1000 0.8
pen up 1000 1000
side off
wait 700
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
expect annotate.luma not 255
shot 01-drawn-again
pen line 300 1334 1500 1334 40 0.8 0.8
side on
pen line 300 1334 1500 1334 40 0.8 0.8
side off
wait 700
expect editor.erase_strokes is 2
expect editor.stroke_look is white pencil
expect editor.live_matches_render is 1
shot 02-white-over-the-text
pentap panel library
wait 600
expect screen is library
quit
