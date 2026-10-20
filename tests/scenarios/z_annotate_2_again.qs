# root: z_annotate_1_new
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-12 import/.annotate
# pdfs: 1
# Run after z_annotate_1_new on the same root: the same page of the same book again. Its notebook
# (found by notebook.ini's source=) has the page already (source.ini maps book page 12 of 96 to it),
# so it opens as it is, with the ink written on it before: no second page, no second notebook.
wait 300
shot 01-the-same-page
expect screen is editor
expect annotate.result is opened
expect annotate.leftover is 0
expect editor.pages is 1
expect editor.strokes is 1
expect annotate.role is document
expect annotate.book_page is 12/96
pentap 1768 1566
wait 600
expect screen is library
expect library.items is 1
quit
