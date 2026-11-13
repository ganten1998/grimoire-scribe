# settings: status_mode=1
# settings: link_areas=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/links import/.book
# pdfs: 1
# rust-only
# KOReader's "Open this book in Grimoire" on a 3-page planner-like PDF, reading page 2, with the links its
# plugin sent (links.txt: page 1 to pages 2 and 3, page 2 to pages 1 and 3, page 3 to page 1; a link to its own
# page and one to a page the book lacks are refused). The notebook opens with its tabs working: they are part of
# the page. A tap follows one; the eye draws their boxes dotted and unnumbered (the Links bar lists no row for
# them, one "From the PDF" row opens them grouped by target); Clear page keeps them (nothing to clear on a page
# of tabs and no ink: no undo step); a finger held on one arms Select, as on paper; the Graph draws none of them.
wait 600
shot 01-page-2
expect screen is editor
expect annotate.book is done
expect annotate.book_added is 3
expect editor.page is 2
# (page 2: its two tabs; the notebook: five, the two refused ones are not there)
expect editor.doclinks is 2 5
expect editor.links is 0 0
expect editor.doclink.1 is p1 200 300 400 300
expect editor.doclink.2 is p3 800 1200 300 300
expect editor.doclink.3 is
# a tap on the first tab goes to page 1; a tap off every tab does nothing
tap 1500 2000
wait 200
expect editor.page is 2
tap 400 450
wait 300
expect editor.page is 1
expect editor.doclinks is 2 5
# the eye is on (its setting): it draws both tabs, none of them numbered
expect settings.link_areas is 1
expect editor.doc_areas is 2
expect editor.areas is 0
shot 02-eye
# the Links bar: no row of its own links, one "From the PDF (2)" row; it opens as two chips, one per target
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 0
expect editor.bar.docs is 2
expect editor.bar.docs.row is 1
expect editor.bar.docs.view is list
shot 03-bar-row
tap 400 2200
wait 300
expect editor.bar.docs.view is docs
expect editor.bar.docs.entries is 2
expect editor.bar.docs.1 is p2
expect editor.bar.docs.2 is p3
shot 04-bar-docs
# X goes back to the page's own rows, a second X closes the bar
tap 1761 2046
wait 300
expect editor.bar.docs.view is list
expect editor.bar is open
tap 1761 2046
wait 300
expect editor.bar is 0
# a finger held on a tab arms Select, as on paper (a hand-made link would open its card): no card, the lasso is armed
touch down 600 450
wait 800
expect editor.link_card is 0
expect editor.select is lasso
touch up 600 450
wait 100
tap 1300 1900
wait 100
# Clear page on a page of tabs and no ink: its button has nothing to clear (disabled), no undo step; the tabs stay
expect editor.strokes is 0
expect editor.doclinks is 2 5
expect editor.undo_kind is
# ink on a tab, then Clear: the ink goes, the tabs stay
pen line 250 400 550 420 30 0.6 0.6
pen leave
wait 300
expect editor.strokes is 1
tap panel clear
wait 300
expect editor.strokes is 0
expect editor.undo_kind is clear
expect editor.doclinks is 2 5
tap panel undo
wait 300
expect editor.strokes is 1
expect editor.doclinks is 2 5
# a lasso around the ink in the tab's box never picks up the tab: the ink only
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
pen down 150 250
pen move 650 250
pen move 650 650
pen move 150 650
pen up 150 270
wait 200
expect editor.select is selected
expect editor.selected is 1
expect editor.sel_links is 0
expect editor.doclinks is 2 5
# the selection's corner handle scales the ink (2.1.0-beta.2, nebuff's resize) and leaves the tab where it was
expect editor.view is 1.00 0 0
expect editor.sel_box is 235 385 331 51
pen down 566 436
pen move 650 470
pen move 731 512
wait 150
expect editor.select is dragging
pen up 731 512
wait 200
expect editor.select is selected
expect editor.undo_kind is lasso
expect editor.selected is 1
expect editor.sel_links is 0
expect editor.sel_box not 235 385 331 51
expect editor.doclinks is 2 5
expect editor.live_matches_render is 1
# undo puts the ink back, the tabs as they were
tap panel select.done
wait 100
pentap panel undo
wait 200
expect editor.strokes is 1
expect editor.doclinks is 2 5
