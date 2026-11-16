# settings: status_mode=1
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/tabs books
# generate: tests/book_v2_request.py --dir books --book 1fba66e74df2ca9847e3725e1571df47 --page 2 --pages 4
# pdfs: 1
# rust-only
# A book opened in KOReader before 2.2 (2.2, request.ini version 2): z_pdf_11's notebook holds the book pages
# KOReader drew through version 1, 1 and 3 of the tabs planner. Now the same planner goes by its path, the reader on
# page 2 (the PDF in a folder of the storage, read in place). The PDF import fills the notebook by its key with pages
# 2 and 4 in book order, and the notebook opens at book page 2. KOReader's pages stay as they were (the page there
# before page 2 is book page 1, KOReader's); no second notebook is made.
waitfor screen is editor
shot 01-page-2
expect annotate.book is done
expect annotate.book_version is 2
expect toast is Added 2 book pages.
expect editor.pages is 4
expect editor.page is 2
expect annotate.book_page is 2/4
expect annotate.role is document
pentap panel previous
wait 300
expect editor.page is 1
expect annotate.book_page is 1/4
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect library.items is 1
quit
