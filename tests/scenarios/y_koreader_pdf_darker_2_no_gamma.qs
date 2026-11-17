# settings: status_mode=1
# settings: pdf_gamma=1.5
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/pick documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 15ada3f23cc724e9d20d62eac07166c4 --page 1 --pages 2
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" on a PDF by its path (2.2) with no gamma= line (an older plugin, or a reader
# at KOReader's own contrast): Grimoire's darker lines setting decides (issue #173), here Medium, so the PDF import
# draws the book's gray pages at 1.5. The notebook (the planner of z_pdf_7) opens at its page 1, at the level
# z_pdf_darker_1_row's page 1 has at 1.5, between Off (z_pdf_darker_3_off) and 2.0 (y_koreader_pdf_darker_1_gamma),
# and says its level.
wait 300
expect annotate.book_version is 2
waitfor screen is editor
expect annotate.book is done
expect editor.page is 1
expect annotate.gamma is 1.5
expect annotate.luma is 241
shot 01-page-1-medium
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
quit
