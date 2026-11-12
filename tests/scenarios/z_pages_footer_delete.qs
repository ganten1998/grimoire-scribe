# rust-only
# Pages: ticked pages are deleted from the bottom row's Delete (issue #18), with no held card. The
# unticked page stays, the view stays, and its ticks clear; the held page's card still has its own
# Delete (j_pages and z_page_management).
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen curve 300 500 40 60 0.7 1
pentap panel next
wait 200
expect editor.page is 2
pen curve 300 500 40 60 0.7 2
pentap panel next
wait 200
expect editor.page is 3
pen curve 300 500 40 60 0.7 3
wait 200
expect editor.pages is 3
pentap panel pages
wait 400
expect editor.modal is pages
# A page tapped again (past the double tap's 450 ms) is unticked, with no Multi-select needed.
tap panel pages.page.1
wait 600
expect editor.ticked is 1
tap panel pages.page.1
wait 400
expect editor.ticked is 0
tap panel pages.multi
tap panel pages.page.1
tap panel pages.page.3
wait 200
expect editor.ticked is 2
# The bottom row's Delete, with no card: both ticked pages go; the unticked page 2 stays.
tap panel pages.delete
wait 400
shot 01-deleted-from-bottom-row
expect editor.modal is pages
expect editor.pages is 1
expect editor.ticked is 0
# Page 2, the one that was not ticked, is the page left: its ink stays (the grid numbers it 1 now).
expect editor.strokes is 1
tap panel pages.back
wait 300
expect editor.modal is none
expect editor.page is 1
quit
