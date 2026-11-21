# settings: status_mode=1
# root: z_pdf_14_fill_4_held
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_BOOK_STEP_PAGES=1
# copy: tests/fixtures/book/fill_job import/.book
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" adds a page to the book's notebook while a fill of it waits (2.2): the
# two pages z_pdf_14 made (book pages 2 and 4) wait in their work folder, and KOReader sends book pages 1 and 2,
# reading page 2. Its job looks at one page a tick (GRIMOIRE_BOOK_STEP_PAGES), so it runs over two ticks with the
# Library in front and at rest. No fill's commit begins while a book job runs: the job adds page 2, the editor
# opens on it, and back in the Library the fill adds page 4 only, the one still missing. (A commit begun during
# the job would have its pages taken out of the list again by the job's own notebook.ini.)
wait 600
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 1
expect editor.pages is 3
expect editor.page is 2
pentap panel library
wait 600
waitbg
wait 300
expect screen is library
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 4
quit
