# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-12 import/.annotate
# pdfs: 1
# A page KOReader sent, looked at and left without a stroke: the notebook made for it at this start
# is a change all the same, so its PDF is written as it closes, and its card in the Library is the
# page (its thumbnail, made as the editor leaves the page).
# rust-only
wait 300
expect screen is editor
expect annotate.result is made
expect editor.strokes is 0
pentap panel library
wait 600
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
expect library.card.1.luma not 255
quit
