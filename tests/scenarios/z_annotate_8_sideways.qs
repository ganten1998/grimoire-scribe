# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/annotate/sideways import/.annotate
# pdfs: 1
# Page 31 of 340 of a reflowable book read sideways: KOReader's picture is its screen, 2480 x 1860,
# not the page's size. The page is made at once and the editor opens on it, white, while the picture
# is fitted to the upright page in the background (each background job waits 1.5 s here); once it is
# done the page is drawn again with it, and ink goes on it as on any page.
# rust-only
wait 300
shot 01-white-while-fitted
expect screen is editor
expect annotate.result is made
expect annotate.role is document
expect annotate.book_page is 31/340
expect annotate.luma is 255
waitbg
wait 300
shot 02-fitted
expect annotate.luma not 255
expect screen is editor
pen line 300 1000 1300 1050 40 0.6 0.6
wait 700
expect editor.strokes is 1
expect editor.live_matches_render is 1
pentap panel library
wait 600
expect screen is library
expect library.items is 1
quit
