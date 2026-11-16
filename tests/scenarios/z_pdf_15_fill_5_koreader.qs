# settings: status_mode=1
# root: z_pdf_14_fill_4_held
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/fill import/.book
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" while a fill of the book's notebook waits (2.2): the two pages z_pdf_14
# made wait in their work folder. KOReader's request wins: the book opens on the reader's page as it does with no
# fill (both pages it sends are there already), with no toast over it, and the fill's pages go in only once the
# notebook is closed and the Library is in front, with their own toast.
wait 600
expect screen is editor
expect annotate.book is done
expect editor.pages is 2
expect editor.page is 1
expect toast is
pentap panel library
waitfor toast is Added 2 book pages.
waitbg
wait 300
expect library.items is 1
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 4
quit
