# settings: status_mode=1
# settings: home=files/documents
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 1
# rust-only
# KOReader's PDF hand-over (2.2, issue #160) at a start whose home is inside Files (documents): the Library's
# cards stay in front (home waits for the notebook KOReader hands over), the notebook opens at the reader's page by
# itself, and closing it opens Files at home, with nothing said.
wait 300
expect screen is library
expect annotate.book is running
expect files.open is 0
expect library.home.pending is 1
waitfor screen is editor
expect annotate.book is done
expect editor.page is 3
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect files.open is 1
expect files.place is documents
expect files.at_home is 1
expect toast not Home folder not found.
quit
