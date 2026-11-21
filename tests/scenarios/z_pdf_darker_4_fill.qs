# settings: status_mode=1
# settings: pdf_gamma=2.0
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/tabs import
# pdfs: 1
# rust-only
# A fill draws the pages it adds at its notebook's own gamma (2.2, issues #123 and #173), whatever the setting says.
# The notebook KOReader made in z_pdf_11 (book pages 1 and 3 of the tabs planner) has no gamma line: its book is at
# Off. With darker lines at Strong, the tabs planner dropped in import/ fills it as z_pdf_12 does, and the two pages it
# adds (book pages 2 and 4) are drawn at Off, the levels pinned here being those z_pdf_12's fill draws.
waitfor toast is Added 2 book pages.
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 4
expect annotate.gamma is 1.0
pentap panel next
wait 400
expect editor.page is 2
expect annotate.luma is 251
pentap panel next
wait 400
pentap panel next
wait 400
expect editor.page is 4
expect annotate.luma is 251
pentap panel library
wait 600
expect screen is library
quit
