# rust-only
# generate: tests/pages_paper_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/pages-redo-keys .
# A batch's paper (the owner, 2026-11-03: "Batch added pages should have whichever page layout the SELECTED
# page has - and fall back to the open page if none are selected."): a batch goes after the last page ticked,
# else after the open page, and takes that page's paper. With several ticked, the last ticked in page order
# (the one the batch goes after). A tap on New page is a batch of one: the same rule. The toolbar's count pad
# goes after the open page, on its paper. The batch's Redo writes the same pages, on the same paper.
# Papers: page 1 blank (4), 2 wide rule (2, open), 3 dots (10), 4 grid (11), 5 medium rule (1).
wait 300
tap library folder.Void
tap library notebook.A
expect editor.page is 2
expect editor.paper is 2
tap panel pages
waitbg
wait 200
# Page 3 (dots) ticked, New page held, 2: after page 3, on dots
tap panel pages.multi
tap panel pages.page.3
expect editor.ticked is 1
touch down 328 2395
wait 700
touch up 328 2395
wait 100
expect editor.pages.adding is 1
tap panel pages.key.2
tap panel pages.key.done
wait 300
expect editor.page_ids is 1,2,3,6,7,4,5
expect toast is
expect editor.pages.undo.kind is add
# Undone and redone (Ctrl+Z, Ctrl+Y): the same ids at the same places
key 44 1
wait 200
expect editor.page_ids is 1,2,3,4,5
expect editor.pages.redo.kind is add
key 21 1
wait 200
expect editor.page_ids is 1,2,3,6,7,4,5
expect toast is
shot 01-redone-batch
# Pages 2 (wide rule) and 7 (id 5, medium rule) ticked (Multi-select still on): a tap on New page goes after
# page 7, on medium rule
expect editor.pages.multi is 1
tap panel pages.page.2
tap panel pages.page.7
expect editor.ticked is 2
tap panel pages.new
wait 300
expect editor.page_ids is 1,2,3,6,7,4,5,8
expect editor.ticked is 0
# Nothing ticked: after the open page (page 2, wide rule), on wide rule
tap panel pages.new
wait 300
expect editor.page_ids is 1,2,9,3,6,7,4,5,8
shot 02-papers
# Each page's paper, opened: 5 and 6 (ids 6, 7) dots, 9 (id 8) medium rule, 3 (id 9) wide rule
tap panel pages.multi
tap panel pages.page.5
tap panel pages.page.5
wait 300
expect editor.modal is none
expect editor.page is 5
expect editor.paper is 10
tap panel pages
wait 200
tap panel pages.page.6
tap panel pages.page.6
wait 300
expect editor.page is 6
expect editor.paper is 10
tap panel pages
wait 200
tap panel pages.page.9
tap panel pages.page.9
wait 300
expect editor.page is 9
expect editor.paper is 1
tap panel pages
wait 200
tap panel pages.page.3
tap panel pages.page.3
wait 300
expect editor.page is 3
expect editor.paper is 2
# The toolbar's count pad (key N): after the open page (page 3, wide rule), on its paper
key 49 0
wait 200
expect editor.addpad is 1
tap panel addpad.key.1
tap panel addpad.key.done
wait 300
expect editor.page_ids is 1,2,9,10,3,6,7,4,5,8
expect editor.page is 3
tap panel next
wait 300
expect editor.page is 4
expect editor.paper is 2
quit
