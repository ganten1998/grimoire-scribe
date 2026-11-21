# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" on a PDF (2.2, request.ini version 2): to a launcher whose Capabilities
# line says pdf, the plugin names the PDF it has open (documents/Tabs Planner.pdf, the reader on page 3 of 4) by its
# path and draws nothing. Grimoire takes the path through the storage walk to the PDF import, as a picked file, and
# takes the request at once. The import runs as a dropped PDF's does (each slice held back 1.5 s here, one page a
# slice): the import's own notice counts its pages, and the Library stays usable, so New notebook opens and closes
# meanwhile. When the import ends with the Library's cards in front, the notebook it made, named after the book,
# opens at the reader's page, book page 3, a document page on the import's own picture, with the toast for the pages
# added.
wait 300
expect screen is library
expect annotate.book is running
expect annotate.book_version is 2
expect annotate.book_leftover is 0
waitfor notice.key is import
expect notice has Importing: page
tap library add
tap library add.new-notebook
wait 300
expect library.sheet is new
tap library done
wait 300
expect library.sheet is none
expect annotate.book is running
waitfor screen is editor
shot 01-page-3
expect annotate.book is done
expect toast is Added 4 book pages.
expect editor.pages is 4
expect editor.page is 3
expect annotate.book_page is 3/4
expect annotate.role is document
expect annotate.luma not 255
pentap panel library
wait 600
waitbg
wait 300
shot 02-library
expect screen is library
expect library.items is 1
quit
