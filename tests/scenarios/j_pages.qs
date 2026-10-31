# settings: status_mode=1
# Pages view: previews, tick pages 2 and 4, delete (a held page's card); open a page from its preview; select all + delete
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
expect editor.page is 2
pen curve 300 500 40 60 0.7 2
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 3
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 4
pentap panel next
wait 200
expect editor.page is 5
expect editor.pages is 5
pen curve 300 500 40 60 0.7 5
wait 200
expect editor.strokes is 1
pentap panel pages
wait 400
shot 01-pages-view
expect editor.modal is pages
expect editor.ticked is 0
tap panel pages.multi
tap panel pages.page.2
expect editor.ticked is 1
tap panel pages.page.4
wait 200
shot 02-ticked-2-and-4
expect editor.ticked is 2
# Delete is on a held page's card: page 4's (ticked), so it deletes both (tile 4's centre is 318 1249)
touch down 318 1249
wait 700
touch up 318 1249
expect editor.pages.context is 4
tap panel pages.delete
expect toast has Deleted 2 pages
wait 400
shot 03-after-delete
expect editor.pages is 3
expect editor.modal is pages
expect editor.ticked is 0
tap panel pages.page.1
tap panel pages.page.1
wait 400
shot 04-opened-page-1
expect editor.modal is none
expect editor.page is 1
expect editor.strokes is 1
pentap panel pages
wait 400
expect editor.modal is pages
tap panel pages.all
wait 200
expect editor.ticked is 3
touch down 318 551
wait 700
touch up 318 551
expect editor.pages.context is 1
tap panel pages.delete
expect toast has Deleted 3 pages
wait 400
shot 05-all-deleted
expect editor.pages is 1
expect editor.strokes is 0
tap 125 95
wait 300
shot 06-back-in-editor
expect editor.modal is none
expect editor.page is 1
quit
