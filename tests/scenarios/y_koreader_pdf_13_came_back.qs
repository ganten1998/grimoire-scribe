# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1000
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 0
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2); the user opens a notebook of their own
# meanwhile and is back on the Library before the import ends. Having gone on to something else, they are taken
# nowhere: the Library stays in front, the book's notebook arrives as a card (as a dropped PDF's does, with no toast
# over it), and that card opens at the page the reader was on, book page 3.
wait 300
expect screen is library
expect annotate.book is running
tap library new
wait 300
tap library create
wait 500
expect screen is editor
pentap panel library
wait 600
expect screen is library
expect annotate.book is running
waitfor annotate.book is done
wait 300
waitbg
shot 01-library
expect screen is library
expect library.sheet is none
expect toast is
expect library.items is 2
expect library.card.1.name is Tabs Planner
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 4
expect editor.page is 3
expect annotate.book_page is 3/4
quit
