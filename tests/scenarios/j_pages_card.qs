# settings: status_mode=1
# env: GRIMOIRE_CHECK_FIT=1
# The Pages view's held-page card: Open, Add a link, Rename, Delete and Cancel; the bottom row no longer
# has a Delete. Rename names the held page (not the open one) on the naming keyboard over the grid, as
# name-page names the open page; its Cancel gives the grid back. Delete deletes the ticked pages when the
# held page is one of them ("Delete (2)"), else the held page alone; Cancel keeps the ticks.
# rust-only
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
pen curve 300 500 40 60 0.7 2
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 3
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 4
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 5
wait 200
expect editor.pages is 5
expect editor.page is 5
pentap panel pages
wait 400
expect editor.modal is pages
shot 01-bottom-row
# Held: page 2's card (tile 2's centre upright is 930 551)
touch down 930 551
wait 700
expect editor.pages.context is 2
touch up 930 551
expect editor.pages.context is 2
shot 02-card
# Rename: the naming keyboard for page 2, empty (it has no name yet), over the grid
tap panel pages.rename
wait 200
expect editor.pages.context is 0
expect editor.modal is pages
expect editor.keyboard is Page name
expect editor.text is
shot 03-rename-keyboard
tap 221 2045
expect editor.text is a
# Done names page 2; the open page (5) keeps no name; the grid stays
tap 1643 2365
wait 200
expect editor.keyboard is
expect editor.modal is pages
expect editor.tile.2 is a
expect editor.page_name is
expect editor.index_rows is 1
shot 04-renamed
# Rename again starts from the page's name; its Cancel leaves it as it was
touch down 930 551
wait 700
touch up 930 551
tap panel pages.rename
wait 200
expect editor.text is a
tap 221 2045
expect editor.text is aa
tap 125 2365
wait 200
expect editor.keyboard is
expect editor.modal is pages
expect editor.tile.2 is a
# Pages 4 and 5 ticked; page 1 held (not ticked): its card's Cancel keeps the ticks
tap panel pages.multi
tap panel pages.page.4
tap panel pages.page.5
expect editor.ticked is 2
touch down 318 551
wait 700
touch up 318 551
expect editor.pages.context is 1
shot 05-card-unticked-page
tap panel pages.cancel
wait 200
expect editor.pages.context is 0
expect editor.ticked is 2
# ... and its Delete deletes page 1 alone
touch down 318 551
wait 700
touch up 318 551
tap panel pages.delete
expect toast has Deleted 1 page
wait 400
expect editor.modal is pages
expect editor.pages is 4
expect editor.ticked is 0
expect editor.tile.1 is a
expect editor.page is 4
# Pages 3 and 4 ticked, page 3 held: Delete (2) deletes both
tap panel pages.page.3
tap panel pages.page.4
expect editor.ticked is 2
touch down 1542 551
wait 700
touch up 1542 551
expect editor.pages.context is 3
shot 06-card-delete-two
tap panel pages.delete
expect toast has Deleted 2 pages
wait 400
shot 07-after-delete
expect editor.pages is 2
expect editor.ticked is 0
expect editor.tile.1 is a
expect editor.page is 2
# Sideways: the bottom row and the card (tile 1's centre sideways is 421 448)
rotate 1
wait 400
expect editor.modal is pages
shot 08-sideways-bottom-row
touch down 421 448
wait 700
touch up 421 448
expect editor.pages.context is 1
shot 09-sideways-card
tap panel pages.rename
wait 200
expect editor.keyboard is Page name
expect editor.text is a
shot 10-sideways-rename-keyboard
quit
