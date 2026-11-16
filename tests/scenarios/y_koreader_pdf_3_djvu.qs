# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/three import/.book
# pdfs: 1
# rust-only
# A DjVu (as a comic or an XPS) still goes by version 1 to a Grimoire that takes PDFs by their path (2.2): KOReader
# draws every page the book's notebook lacks and names those pictures in its request (version 1), as before. This
# Grimoire takes it as before: a notebook of KOReader's pictures, opened on the reader's page, and no PDF import.
wait 600
expect annotate.book_version is 1
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 3
expect toast is Added 3 book pages.
expect editor.pages is 3
expect editor.page is 2
expect annotate.book_page is 2/3
expect annotate.role is document
pentap panel library
wait 600
waitbg
expect library.items is 1
quit
