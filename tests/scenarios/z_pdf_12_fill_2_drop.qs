# settings: status_mode=1
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/tabs import
# pdfs: 1
# rust-only
# The tabs planner itself dropped in grimoire/import/ (2.2): its book key is that of the notebook KOReader made in
# z_pdf_11, which lacks book pages 2 and 4. The PDF fills it instead of saying "PDF already imported.": the two
# pages come in book order (2 before 3, 4 last), the links that reach them are added, and the tabs KOReader sent
# are not added a second time (page 1 keeps its two links: KOReader's to page 3, as KOReader drew it, and the one
# to page 4). The pages with no name take the outline's (Cover, Notes). No second notebook is made.
waitfor toast is Added 2 book pages.
shot 01-filled
waitbg
wait 300
expect library.items is 1
tap 326 608
wait 700
shot 02-page-1
expect screen is editor
expect editor.pages is 4
expect editor.page is 1
expect editor.doclinks is 2 5
expect editor.doclink.1 is p3 1521 321 320 160
expect editor.doclink.2 is p4 80 2240 320 160
expect editor.doclink.3 is
expect editor.page_name is Cover
# page 2, one of the pages added: its tab goes to page 4
pentap panel next
wait 400
expect editor.page is 2
expect editor.doclinks is 1 5
expect editor.doclink.1 is p4 1520 320 320 160
# page 3, KOReader's: its one tab, not two
pentap panel next
wait 400
expect editor.page is 3
expect editor.doclinks is 1 5
expect editor.doclink.1 is p1 1521 321 320 160
expect editor.page_name is Notes
pentap panel library
wait 600
expect screen is library
quit
