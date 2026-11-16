# settings: status_mode=1
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=4000
# copy: tests/fixtures/pdf/tabs import
# pdfs: 1
# rust-only
# While a fill is under way its notebook does not open (2.2). The planner of z_pdf_12 again, its slices slowed: once
# the probe has decided the fill (the staying notice counts its two pages), a tap on the notebook's card says how
# far the import is and stays in the Library; once the pages are in, the card opens the notebook, filled.
waitfor notice is Importing: page 1 of 2
tap 326 608
wait 300
expect screen is library
expect toast is Importing: page 1 of 2
shot 01-refused
waitfor toast is Added 2 book pages.
waitbg
wait 300
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 4
quit
