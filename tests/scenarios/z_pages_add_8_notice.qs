# rust-only
# generate: tests/add_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# "Adding N pages..." for a big batch of new pages (the owner, 2026-11-04: "Show "Adding N pages..." only for big
# batches (100 or more pages, or when adding takes longer than about 2 seconds), and take it down when done. Small
# batches stay silent."). Nothing reaches the panel while the pages are written, so a batch's time is told from its
# count before it starts (about 40 ms a page on a Kindle, on any paper: the notice from 51 pages), and the notice
# goes up in the refresh that takes the count pad down and comes down in the one that shows the pages. A failed
# batch's own toast takes its place. The toolbar's count pad and the Pages view's alike.
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting; the clock
# corner held still)
wait 3600
clock 1700000000
tap library folder.Void
tap library notebook.S
expect editor.page_ids is 1,2
expect editor.add_notices is 0
pentap panel next
wait 300
pen leave
expect editor.page is 2
# The toolbar's + held on the last page: 100 pages after page 2, the notice over the page while they are written
# (with the pad's going), gone with the refresh that shows the page again
touch down 1768 1330
wait 700
touch up 1768 1330
wait 100
expect editor.addpad is 1
tap panel addpad.key.1
tap panel addpad.key.0
tap panel addpad.key.0
wait 100
expect editor.text is 100
expect display.presents not -1
tap panel addpad.key.done
wait 300
expect editor.addpad is 0
expect editor.pages is 102
expect editor.page is 2
expect editor.add_notices is 1
expect toast is
expect display.presents is ui 20 1310 1820 1150, ui 20 1212 1820 1248
# The Pages view's New page held: 10 pages, no notice
tap panel pages
waitbg
wait 200
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
tap panel pages.key.1
tap panel pages.key.0
wait 100
expect editor.pages.text is 10
tap panel pages.key.done
wait 300
expect editor.pages.adding is 0
expect editor.modal is pages
expect editor.pages is 112
expect editor.add_notices is 1
expect toast is
# 50 pages, about 2 s on a Kindle: none
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
tap panel pages.key.5
tap panel pages.key.0
wait 100
expect editor.pages.text is 50
tap panel pages.key.done
wait 300
expect editor.pages is 162
expect editor.add_notices is 1
expect toast is
# 51 pages, over 2 s: the notice while they are written, down once they are added
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
tap panel pages.key.5
tap panel pages.key.1
wait 100
expect editor.pages.text is 51
tap panel pages.key.done
wait 300
expect editor.pages is 213
expect editor.add_notices is 2
expect toast is
# 100 pages: the notice in the refresh that takes the pad down, before a file is written, and gone in the one
# that shows the pages (the previews' tiles may follow, as they come)
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
tap panel pages.key.1
tap panel pages.key.0
tap panel pages.key.0
wait 100
expect editor.pages.text is 100
expect display.presents not -1
tap panel pages.key.done
wait 300
expect editor.pages is 313
expect editor.add_notices is 3
expect toast is
expect display.presents has ui 0 0 1860 2480, ui 0 0 1860 2480
# 100 pages that cannot be written (the storage full): the notice, then "Couldn't add pages." in its place, and
# no page added
storage full
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
tap panel pages.key.1
tap panel pages.key.0
tap panel pages.key.0
wait 100
tap panel pages.key.done
wait 300
storage ok
expect editor.pages is 313
expect editor.add_notices is 4
expect toast is Couldn't add pages.
quit
