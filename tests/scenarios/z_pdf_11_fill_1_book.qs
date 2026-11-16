# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/fill import/.book
# pdfs: 1
# rust-only
# The first of three (2.2: the same PDF again fills its notebook). KOReader's "Open this book in Grimoire" on the
# tabs planner of z_pdf_9_tabs (its book key), reading page 1: the plugin sent book pages 1 and 3 only, and the
# links it found on them (its boxes a pixel off the import's: two engines draw them). The book's notebook holds
# those two pages and two tabs, page 1 to page 3 and page 3 to page 1; the link to page 4, a page it does not
# hold, is refused. z_pdf_12 and z_pdf_13 drop the PDF itself in import/ on this notebook.
wait 600
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 2
expect editor.pages is 2
expect editor.page is 1
expect editor.doclinks is 1 2
expect editor.doclink.1 is p2 1521 321 320 160
expect editor.page_name is
pentap panel library
wait 600
expect screen is library
expect library.items is 1
quit
