# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=2 GRIMOIRE_BG_DELAY_MS=500
# copy: tests/fixtures/pdf/planner import
# pdfs: 0
# A planner PDF in grimoire/import/ (copied in while Grimoire was closed) becomes a notebook of its pages with
# no KOReader: the staying notice says how far it is at the bottom of the Library ("Importing: page 3 of 7"),
# the cards stay usable, and when it ends the card "2027 Planner" is there and no toast says so (the card does).
# The PDF has moved to import/done/. Its pages are the PDF's pages, fitted and centred on white.
# (GRIMOIRE_BG_DELAY_MS makes each slice take half a second, so the notice is up long enough to see.)
# rust-only
waitfor notice has Importing: page
shot 01-importing
expect notice.key is import
expect screen is library
waitbg
wait 300
shot 02-imported
expect notice is
expect toast is
expect library.items is 1
expect library.card.1 is thumb
tap 326 608
wait 700
shot 03-page-1
expect screen is editor
expect editor.pages is 7
expect editor.page is 1
pentap panel next
wait 400
shot 04-page-2
expect editor.page is 2
pentap panel library
wait 600
expect screen is library
quit
