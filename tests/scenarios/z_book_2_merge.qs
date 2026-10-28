# settings: status_mode=1
# root: z_annotate_3_order
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/merge import/.book
# pdfs: 1
# "Open this book in Grimoire" on the book whose notebook already holds book pages 5 and 12 (with
# ink on 12), reading page 13. The request names pages 11, 12 and 13: page 12 is there already and
# stays as it is, ink and all; 11 goes before it and 13 after it, in book order. The editor opens
# on page 13, the notebook's fourth.
# rust-only
wait 600
shot 01-page-13
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 2
expect annotate.book_leftover is 0
expect editor.pages is 4
expect editor.page is 4
expect annotate.book_page is 13/96
pentap panel library
wait 600
expect screen is library
expect library.items is 1
quit
