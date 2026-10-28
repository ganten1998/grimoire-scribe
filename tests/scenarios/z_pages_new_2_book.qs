# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-12 import/.annotate
# An annotated book page keeps its source label. New pages are blank notes; copying its picture
# creates another note page and never takes over the original book-page mapping.
# rust-only
wait 500
expect screen is editor
expect editor.pages is 1
pentap panel pages
wait 200
expect editor.tile.1 is Page 1 (open) | p. 12
tap 450 2395
wait 200
expect editor.page is 2
expect editor.pages is 2
expect editor.paper is 4
expect editor.strokes is 0
pentap panel pages
wait 200
expect editor.tile.1 is Page 1 | p. 12
expect editor.tile.2 is Page 2 (open)
tap 396 296
tap 1410 2395
wait 200
expect editor.page is 2
expect editor.pages is 3
expect annotate.role is document
pentap panel pages
wait 200
expect editor.tile.1 is Page 1 | p. 12
expect editor.tile.2 is Page 2 (open)
expect editor.tile.3 is Page 3
shot 01-book-and-notes
tap 251 340
wait 200
expect editor.page is 1
expect annotate.book_page is 12/96
quit
