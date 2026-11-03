# rust-only
# generate: tests/drag_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# A row picked in the Find sheet over the Pages view when the open page cannot be saved (the store full, after a
# delete in the view left page 1 to write again): the view closes on the page, which stays, said once, in one full
# refresh of the screen (go_to_page's own; the row's press its fast one before it).
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library folder.Void
tap library notebook.A
tap panel pages
waitbg
wait 300
expect editor.modal is pages
storage full
tap panel pages.multi
tap panel pages.page.2
expect editor.ticked is 1
touch down 930 500
wait 700
touch up 930 500
expect editor.pages.context is 2
tap panel pages.delete
wait 300
expect editor.modal is pages
expect editor.unsaved is 1
tap panel pages.goto
wait 300
tap 841 1725
expect editor.find.1 is Go to page 5
expect display.presents not -1
tap 930 1390
wait 300
expect editor.find is 0
expect editor.modal is none
expect editor.page is 1
expect toast has Could not save this page
expect display.presents is fast 36 1328 1788 124, full 0 0 1860 2480
storage ok
quit
