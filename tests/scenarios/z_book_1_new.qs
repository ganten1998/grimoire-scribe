# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/three import/.book
# pdfs: 1
# KOReader's "Open this book in Grimoire" on a 3-page PDF, reading page 2: its plugin drew every
# page into grimoire/import/.book/ with a request naming them and started Grimoire hosted, asked to
# open it (kindle/app/grimoire.sh sets GRIMOIRE_BOOK=1). No notebook is the book's yet: one is made,
# named after its title, with the three pages in book order, each a document page on its picture
# (pages 1 and 2 drawn at the Scribe's page size are their pictures as they are; page 3, drawn
# sideways, is fitted to its page in the background). The editor opens on page 2, and the request
# and its pictures are gone.
# rust-only
wait 600
shot 01-page-2
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 3
expect annotate.book_leftover is 0
expect editor.pages is 3
expect editor.page is 2
expect annotate.role is document
expect annotate.book_page is 2/3
expect annotate.luma not 255
pentap panel next
wait 1500
shot 02-page-3-fitted
expect editor.page is 3
expect annotate.book_page is 3/3
expect annotate.luma not 255
pen line 300 700 1500 760 30 0.7 0.7
wait 300
expect editor.strokes is 1
pentap panel library
wait 600
shot 03-library
expect screen is library
expect library.items is 1
quit
