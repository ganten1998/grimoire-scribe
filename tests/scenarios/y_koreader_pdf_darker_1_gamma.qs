# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/pick documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 15ada3f23cc724e9d20d62eac07166c4 --page 1 --pages 2 --gamma 2.0
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" on a PDF by its path (2.2) from a reader that draws it darker: the plugin's
# version 2 request says gamma=2.0 (issue #173), and with darker lines at Off in Grimoire's settings the PDF import
# draws the book's gray pages at 2.0. The notebook (the planner of z_pdf_7) opens at the reader's page, its page 1,
# at the level z_pdf_darker_2_colour's page 1 has at 2.0 (the planner's page 1, drawn alike), and says its level.
wait 300
expect annotate.book_version is 2
expect settings.pdf_gamma is 1.0
waitfor screen is editor
expect annotate.book is done
expect editor.page is 1
expect annotate.gamma is 2.0
expect annotate.luma is 239
shot 01-page-1-strong
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
quit
