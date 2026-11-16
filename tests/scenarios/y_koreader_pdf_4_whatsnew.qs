# settings: status_mode=1
# settings: whatsnew_seen=1.2.0
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/tabs books
# generate: tests/book_v2_request.py --dir books --book 1fba66e74df2ca9847e3725e1571df47 --page 2 --pages 4
# pdfs: 1
# rust-only
# The first start after an update is KOReader's hand-over of a PDF by its path (2.2, request.ini version 2), and the
# PDF fills a notebook: z_pdf_11's, which holds the two book pages KOReader drew. What's new opens by itself over the
# Library, as after every update. The hand-over holds nothing: the import makes the two pages the notebook lacks
# under the sheet, and they wait, as a fill's pages wait for the Library to be at rest. Dismiss closes What's new as
# it always does; the pages then go in, and the notebook opens at the reader's page, book page 2.
wait 300
expect screen is library
expect library.sheet is about
expect annotate.book is running
expect annotate.book_version is 2
waitbg
wait 1000
expect annotate.book is running
expect screen is library
expect library.sheet is about
tap library about.close
wait 300
expect library.sheet is none
waitfor screen is editor
shot 01-page-2
expect annotate.book is done
expect toast is Added 2 book pages.
expect editor.pages is 4
expect editor.page is 2
expect annotate.book_page is 2/4
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect library.sheet is none
expect library.items is 1
quit
