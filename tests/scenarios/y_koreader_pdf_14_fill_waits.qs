# settings: status_mode=1
# root: z_pdf_14_fill_4_held
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/tabs books
# generate: tests/book_v2_request.py --dir books --book 1fba66e74df2ca9847e3725e1571df47 --page 2 --pages 4
# pdfs: 1
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2) while a fill of the book's notebook waits: the
# two pages z_pdf_14 made from a copy dropped in import/ wait in their work folder. The fill goes in by its own
# rules, with its own toast, as the Library is in front and at rest. The book's own import is of the same book: it
# waits for that notebook rather than make the pages twice, then finds every page there. Nothing is held meanwhile,
# and the notebook opens at the reader's page, book page 2, with the four pages and no page added twice.
wait 200
expect annotate.book_version is 2
waitfor screen is editor
wait 300
shot 01-page-2
expect annotate.book is done
expect editor.pages is 4
expect editor.page is 2
expect annotate.book_page is 2/4
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect library.items is 1
quit
