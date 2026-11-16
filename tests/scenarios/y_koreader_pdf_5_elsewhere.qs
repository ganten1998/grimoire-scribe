# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1000
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 0
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2) and the user does something else while it is
# imported: New notebook, Create, and a notebook of their own is open in the editor when the import ends. Nothing
# opens over it: the import says its own toast, as for a dropped PDF, and the page stays the user's. Back in the
# Library the book's notebook is a card like any other, and it opens at the page the reader was on, book page 3,
# kept as the page it opens at.
wait 300
expect screen is library
expect annotate.book is running
tap library new
wait 300
tap library create
wait 500
expect screen is editor
expect editor.pages is 1
expect annotate.book is running
waitfor annotate.book is done
wait 200
shot 01-own-notebook
expect screen is editor
expect editor.pages is 1
expect toast is Imported notebook (4 pages).
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect library.items is 2
expect library.card.1.name is Tabs Planner
tap 326 608
wait 700
shot 02-page-3
expect screen is editor
expect editor.pages is 4
expect editor.page is 3
expect annotate.book_page is 3/4
quit
