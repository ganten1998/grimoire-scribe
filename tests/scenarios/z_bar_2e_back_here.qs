# rust-only
# The back list never offers the page that shows: a link followed and the pages turned back by hand to where
# it began leave that page on the list, and the bar has no "Back to" chip for it (it used to peek the very
# page on show) and the go-back key says there is nowhere to go (it used to do nothing, silently).
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
tap 600 1100
wait 300
expect editor.page is 3
expect editor.back is 1 1
pentap panel previous
pentap panel previous
expect editor.page is 1
expect editor.back is 1 1
wait 1200
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
# the header has no chip for the page on show (the tap lands where it was: on the bar's empty header)
tap 160 2046
wait 300
expect editor.bar is open
expect editor.bar.peek is 0
tap 1761 2046
wait 300
expect editor.bar is 0
# the go-back key finds nothing, and the list is empty after it
key 48 0
wait 300
expect toast is Nothing to go back to.
expect editor.page is 1
expect editor.back is 0 0
