# rust-only
# generate: tests/drag_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# The Pages view's Go to page... opens the Find sheet over the view, which stays behind it (the owner: "it needs
# its own dialog copy in the pages view that doesn't just drag you out to the notebook while you input"). Its
# Cancel and a tap outside it give the view back as it was (its screen, its ticks), refreshing
# only what the sheet covered; a row picked goes to that page and the view closes on it. New page held opens its
# count pad at 1: Done at once adds one page, a digit replaces the 1.
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen: the stretch counted at Cancel
# ended about 3 s after the start, and a slower runner reached the 3.5 s one inside it, issue #93)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap library folder.Void
tap library notebook.A
tap panel pages
waitbg
wait 200
expect editor.modal is pages
# The second screen, page 11 ticked
pen leave
wait 1100
fpath 1400 1300 600 1320 200
wait 300
expect editor.pages.screen is 2
tap panel pages.multi
wait 100
tap panel pages.page.11
wait 100
expect editor.pages.ticks is 11
# Go to page...: the sheet over the view, the view still open behind it
expect display.presents not -1
tap panel pages.goto
wait 300
expect editor.find is go
expect editor.modal is find
expect editor.pages.screen is 2
expect editor.keyboard is Page number or name
shot 01-find-over-pages
# a number typed: its row
tap 841 1725
expect editor.find.1 is Go to page 5
# Cancel: the view as it was, only what the sheet covered refreshed (its keyboard and its card of rows, one Ui)
expect display.presents not -1
tap 217 2365
wait 300
expect editor.find is 0
expect editor.modal is pages
expect editor.pages.screen is 2
expect editor.pages.ticks is 11
expect editor.page is 1
expect display.presents is ui 20 1312 1820 1148
shot 02-find-cancelled
# a tap outside it (on the grid) closes it too, and is swallowed: no tile picked
tap panel pages.goto
wait 300
expect editor.find is go
tap 930 551
wait 300
expect editor.find is 0
expect editor.modal is pages
expect editor.pages.ticks is 11
# a row picked: that page, the view closed on it, the page it left on the back list
tap panel pages.goto
wait 300
tap 841 1725
expect editor.find.1 is Go to page 5
tap 930 1390
wait 300
expect editor.find is 0
expect editor.modal is none
expect editor.page is 5
expect editor.back is 1 1
# New page held: the count pad at 1, Done at once adds one page after the open one
tap panel pages
wait 300
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
expect editor.pages.text is 1
expect editor.pages.add.done is Add 1 page
expect editor.pages.add.done.on is 1
shot 03-count-pad-at-one
tap panel pages.key.done
wait 300
expect editor.pages.adding is 0
expect editor.page_ids is 1,2,3,4,5,13,6,7,8,9,10,11,12
# Held again: a digit replaces the 1 (4, not 14), - and + count from it
touch down 394 2395
wait 700
touch up 394 2395
wait 100
tap panel pages.key.4
expect editor.pages.text is 4
tap panel pages.key.inc
expect editor.pages.text is 5
tap panel pages.key.cancel
wait 200
expect editor.pages.adding is 0
