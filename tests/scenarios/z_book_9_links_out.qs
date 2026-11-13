# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/links_bad import/.book
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" with a links.txt that does not read (one bad line: the file is taken whole
# or not at all). The pages come, no tab does, and the toast says so: "Added 3 book pages. Some links were left
# out." (the plan's wording; the KOReader route used to say only "Added 3 book pages.").
wait 600
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 3
expect editor.doclinks is 0 0
expect toast is Added 3 book pages. Some links were left out.
pentap panel library
wait 600
expect screen is library
quit
