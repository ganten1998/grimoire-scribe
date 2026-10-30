# settings: status_mode=1
# rust-only
# Go to page or name: the bar's Go to (after staying on the page it shows), the go-to-page key and the Pages
# view's "Go to page..." open the Find sheet, a keyboard along the bottom with its rows above it. A number
# offers "Go to page 3" (a tap on the row goes there, one Full, the page left on the back list), or says the
# notebook has no such page; the go-to-page key again closes it.
# root: z_bar_0_make
wait 3700
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect keyboard.draws not -1
tap panel bar.close
key 34 0
wait 300
expect editor.find is go
expect editor.bar is 0
expect editor.modal is find
expect editor.keyboard is Page number or name
expect editor.find.rows is 0
expect keyboard.draws is 1
shot 01-find-open
tap 487 1725
expect editor.text is 3
expect editor.find.rows is 1
expect editor.find.1 is Go to page 3
shot 02-find-3
expect display.presents not -1
tap 930 1390
wait 300
expect editor.find is 0
expect editor.modal is none
expect editor.page is 3
expect editor.back is 1 1
expect display.presents is fast 36 1328 1788 124, full 0 0 1860 2480
key 48 0
wait 300
expect editor.page is 1
# from the Pages view: the view closes and Find opens over the page, in one Full
pentap panel pages
wait 400
expect editor.modal is pages
expect display.presents not -1
tap 930 2395
wait 300
expect editor.find is go
expect editor.modal is find
expect display.presents is full 0 0 1860 2480
# a number the notebook does not have
tap 1549 1725
expect editor.find.1 is There is no page 9 (5 pages).
expect editor.find.rows is 1
shot 03-find-no-page-9
# Done with no row that is a button leaves it as it is
tap 1643 2365
expect editor.find is go
expect editor.text is 9
# the key closes it and opens it again
key 34 0
wait 300
expect editor.find is 0
expect editor.modal is none
expect editor.page is 1
key 34 0
wait 300
expect editor.find is go
key 34 0
wait 300
expect editor.find is 0
# Cancel closes it too
key 34 0
wait 300
tap 217 2365
wait 300
expect editor.find is 0
expect editor.page is 1
# Done goes to the first row that is a button: 5 (the fifth page), and the page it was on is the back list's
key 34 0
wait 300
tap 841 1725
expect editor.find.1 is Go to page 5
tap 1643 2365
wait 300
expect editor.find is 0
expect editor.page is 5
expect editor.back is 1 1
