# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=400
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 1
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2) and a finger rests on a button of the Library
# (New notebook, held down and not let go) when the import ends. The end waits for nothing the user does: the
# notebook opens at the reader's page, book page 3, with the finger still down. The lift then comes to the page,
# which began nothing with it: no sheet opens and the page stays.
wait 300
expect screen is library
expect annotate.book is running
expect library.ui_at.1738.2358 is add
touch down 1738 2358
waitfor screen is editor
expect annotate.book is done
expect editor.pages is 4
expect editor.page is 3
wait 300
touch up 1738 2358
wait 400
shot 01-page-3
expect screen is editor
expect editor.modal is none
expect editor.page is 3
expect editor.strokes is 0
expect annotate.book_page is 3/4
quit
