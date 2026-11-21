# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1000
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 0
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2) and a sheet of the Library is open when its
# import ends: New notebook, here. No notebook opens under the sheet: the import says its own toast, the sheet
# stays, and once it is closed the book's notebook is the Library's card, which opens at the page the reader was on,
# book page 3.
wait 300
expect screen is library
expect annotate.book is running
tap library add
tap library add.new-notebook
wait 300
expect library.sheet is new
waitfor annotate.book is done
wait 200
shot 01-sheet
expect screen is library
expect library.sheet is new
expect toast is Imported notebook (4 pages).
tap library done
wait 300
waitbg
expect library.sheet is none
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 4
expect editor.page is 3
expect annotate.book_page is 3/4
quit
