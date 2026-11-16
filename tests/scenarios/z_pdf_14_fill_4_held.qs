# settings: status_mode=1
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=4000
# copy: tests/fixtures/pdf/tabs import
# pdfs: 1
# rust-only
# For z_pdf_15 (2.2). The notebook KOReader made in z_pdf_11 is opened before the fill of it is even decided (the
# fill's first slice held back), and stays open while the fill's two pages are made: they wait for it to close.
# Grimoire is left with the notebook still open, so the pages made wait in their work folder for the next start.
wait 300
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 2
waitbg
wait 300
expect screen is editor
expect editor.pages is 2
quit
