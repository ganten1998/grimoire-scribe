# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/book/three import/.book
# pdfs: 1
# A book's notebook (KOReader's "Open this book in Grimoire", z_book_1_new.qs) and the next-page key:
# it turns to the book's last page, and there adds a page as the editor's + does, a note page on
# the notebook's own paper (Blank in a book's notebook), never the book page's picture.
wait 600
expect editor.pages is 3
expect editor.page is 2
key 106 0
wait 1500
expect editor.page is 3
expect editor.pages is 3
expect annotate.book_page is 3/3
key 106 0
wait 400
expect editor.pages is 4
expect editor.page is 4
expect editor.paper is 4
expect annotate.role is
pentap 1768 1566
wait 600
expect screen is library
quit
