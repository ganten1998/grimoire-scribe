# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/colour import
# pdfs: 0
# A PDF with a colour page keeps it: the page is stored as an 8-bit RGB picture, the pages that are gray as gray
# ones (pdfimport.rs; the unit tests read the PNGs). The Scribe shows every page in gray, so the colour page
# opens as the gray of its colours, and the notebook has its three pages.
# rust-only
waitbg
wait 300
expect library.items is 1
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 3
pentap panel next
wait 500
shot 01-colour-page-in-gray
expect editor.page is 2
expect editor.paper not 4
pentap panel library
wait 600
quit
