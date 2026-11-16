# settings: status_mode=1
# root: y_koreader_pdf_2_fill
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# generate: tests/book_v2_request.py --dir books --book 1fba66e74df2ca9847e3725e1571df47 --page 4 --pages 4
# pdfs: 1
# rust-only
# Open this book once more on a PDF whose notebook holds every page (2.2, request.ini version 2):
# y_koreader_pdf_2_fill's notebook, filled by the planner's path. The PDF import finds the PDF imported before and
# says in which notebook; the book job opens that one at the reader's page, book page 4, with no toast, and nothing is
# made or added.
waitfor screen is editor
wait 200
expect annotate.book is done
expect annotate.book_version is 2
expect annotate.book_added is 0
expect toast is
expect editor.pages is 4
expect editor.page is 4
expect annotate.book_page is 4/4
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect library.items is 1
quit
