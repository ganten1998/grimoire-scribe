# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-12 import/.annotate
# settings: eraser_mode=1
# settings: side_button=3
# pdfs: 1
# Notes on a page KOReader sent, then the precise eraser and white ink (the pen's button held) over
# them: they take the ink away down to the page's picture, which stays (the book's text is not
# erased), and the page drawn afresh is what the editor shows (live is what a later look gets).
# rust-only
wait 300
expect screen is editor
expect annotate.result is made
pen line 300 900 1300 950 40 0.6 0.6
wait 700
expect editor.strokes is 1
eraser line 300 925 900 925 30
wait 700
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
side on
pen line 900 935 1300 935 30 0.8 0.8
side off
wait 700
expect editor.erase_strokes is 2
expect editor.stroke_look is white pencil
expect editor.live_matches_render is 1
shot 01-erased-to-the-page
pentap panel library
wait 600
expect screen is library
quit
