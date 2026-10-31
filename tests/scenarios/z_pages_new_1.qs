# settings: status_mode=1
# New page and Copy page in the Pages view. New page adds a blank page of the current page's paper
# after the open page, or after the last page ticked, and opens it; Copy page copies the one page
# ticked, its ink and its paper, right after it, and opens the copy. Neither ever makes a page the
# first. Then the same sideways, and all of it still there once the notebook opens again.
# rust-only
wait 200
tap 965 95
wait 100
tap library paper.1
tap 1564 2254
wait 300
expect screen is editor
expect editor.paper is 1
pen curve 300 500 40 60 0.7 1
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 2
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 3
wait 200
expect editor.pages is 3
expect editor.page is 3
# Page 2 goes onto sheet music: New page after it is page 3, on the current page's sheet music.
pentap panel pages
wait 300
expect editor.modal is pages
tap panel pages.page.2
tap panel pages.page.2
wait 300
expect editor.page is 2
pentap panel paper
wait 200
pentap panel paper.3
wait 300
expect editor.paper is 3
pentap panel pages
wait 300
expect editor.tile.2 is Page 2 (open)
shot 01-pages-new-copy-off
tap panel pages.new
wait 300
expect editor.modal is none
expect editor.page is 3
expect editor.pages is 4
expect editor.strokes is 0
expect editor.paper is 3
# The page that was third is fourth now, with its ink.
pentap panel next
wait 200
expect editor.page is 4
expect editor.strokes is 1
# Pages 1 and 2 ticked: Copy page is off, and New page goes after page 2, the last ticked.
pentap panel pages
tap panel pages.multi
wait 300
tap panel pages.page.1
tap panel pages.page.2
wait 100
expect editor.ticked is 2
shot 02-two-ticked
# Copy remains disabled with multiple pages selected.
tap 592 2395
wait 100
expect editor.modal is pages
expect editor.pages is 4
tap panel pages.new
wait 300
expect editor.modal is none
expect editor.page is 3
expect editor.pages is 5
expect editor.strokes is 0
# Copy page: page 2 (ink on sheet music) ticked alone is copied right after it, as page 3.
pentap panel pages
wait 300
tap panel pages.page.2
wait 100
expect editor.ticked is 1
shot 03-one-ticked-copy-on
tap panel pages.copy
wait 300
expect editor.modal is none
expect editor.page is 3
expect editor.pages is 6
expect editor.strokes is 1
expect editor.paper is 3
expect toast has a copy of page 2
# Ink on the copy stays on the copy.
pen curve 900 1500 40 60 0.7 2
wait 300
expect editor.strokes is 2
pentap panel previous
wait 200
expect editor.page is 2
expect editor.strokes is 1
expect editor.paper is 3
# Page 1 is still the first: the Library's card.
pentap panel pages
wait 300
expect editor.tile.1 is Page 1
tap panel pages.page.1
tap panel pages.page.1
wait 300
expect editor.page is 1
expect editor.strokes is 1
# Sideways: New page after page 1, then a copy of page 1.
rotate 1
wait 300
pentap panel pages
wait 300
expect editor.modal is pages
shot 04-pages-landscape
tap panel pages.new
wait 300
expect editor.modal is none
expect editor.page is 2
expect editor.pages is 7
expect editor.strokes is 0
pentap panel pages
wait 300
tap panel pages.page.1
wait 100
expect editor.ticked is 1
tap panel pages.copy
wait 300
expect editor.page is 2
expect editor.pages is 8
expect editor.strokes is 1
rotate 0
wait 300
# All of it there once the notebook opens again.
pentap panel library
wait 300
expect screen is library
tap 200 400
wait 300
expect screen is editor
expect editor.pages is 8
expect editor.page is 2
expect editor.strokes is 1
quit
