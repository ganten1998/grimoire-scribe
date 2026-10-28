# root: z_annotate_2_again
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-5 import/.annotate
# pdfs: 1
# Another page of the same book, page 5, comes before page 12: a new page in book order, the
# notebook's first now, and the editor opens on it. Page 12 (with its ink) is the second. The new
# first page is what the Library's card shows: its thumbnail, which the editor makes as it leaves
# the page (the one of the page that was first went), and the notebook's PDF is made again.
# rust-only
wait 300
shot 01-page-5-first
expect screen is editor
expect annotate.result is made
expect editor.pages is 2
expect editor.page is 1
expect editor.strokes is 0
expect annotate.role is document
expect annotate.book_page is 5/96
pentap panel next
wait 400
shot 02-page-12-second
expect editor.page is 2
expect annotate.book_page is 12/96
expect editor.strokes is 1
pentap panel library
wait 600
shot 03-library
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
expect library.card.1.luma not 255
quit
