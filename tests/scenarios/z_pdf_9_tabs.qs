# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=2
# copy: tests/fixtures/pdf/tabs import
# pdfs: 0
# A planner PDF with link annotations in grimoire/import/ becomes a notebook whose tabs work: its links are
# document links (the PDF's own tab on every page, and a link by a name in the name tree), their areas the
# annotations' rectangles on the page picture (the fixture's page is a quarter of the Scribe's, so one point is
# 4 px), and the outline names two pages. A tap on the tab goes to the page it names; a tap beside it does not.
# rust-only
waitbg
wait 300
expect notice is
expect library.items is 1
tap 326 608
wait 700
shot 01-page-1
expect screen is editor
expect editor.pages is 4
expect editor.page is 1
# (page 1: its tab and the named link; the notebook: one tab on each of the four pages and the named link)
expect editor.doclinks is 2 5
expect editor.links is 0 0
expect editor.doclink.1 is p3 1520 320 320 160
expect editor.doclink.2 is p4 80 2240 320 160
expect editor.doclink.3 is
# the outline's names are the pages' names (the Links bar and the Index show them)
expect editor.page_name is Cover
# a tap beside the tab does nothing; a tap on it goes to page 3
tap 700 1200
wait 300
expect editor.page is 1
tap 1680 400
wait 400
shot 02-page-3
expect editor.page is 3
expect editor.page_name is Notes
# page 3's tab goes back to page 1
tap 1680 400
wait 400
expect editor.page is 1
# the second link, by its name, reaches the last page
tap 240 2320
wait 400
expect editor.page is 4
pentap panel library
wait 600
expect screen is library
quit
