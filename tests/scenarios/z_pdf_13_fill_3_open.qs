# settings: status_mode=1
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=4000
# copy: tests/fixtures/pdf/tabs import
# pdfs: 1
# rust-only
# A notebook opens while a fill of it is being made, and the fill's pages go in once it is closed (2.2). The
# planner of z_pdf_12 again, its slices slowed: once the probe has decided the fill (the staying notice counts its
# two pages), a tap on the notebook's card opens it, as it is. The pages are made meanwhile and wait; back in the
# Library they go in (the book's notice, then the toast), and the card opens the notebook, filled.
waitfor notice is Importing: page 1 of 2
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 2
shot 01-open
waitbg
wait 300
expect screen is editor
expect editor.pages is 2
pentap panel library
waitfor toast is Added 2 book pages.
waitbg
wait 300
expect screen is library
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 4
quit
