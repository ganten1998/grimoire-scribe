# rust-only
# Two fingers on the page, upright: down opens Pages with one flash, up closes them (the swipe the
# other way takes back what the first did), left and right turn the page and never add one.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pentap panel next
wait 100
pentap panel next
wait 100
expect editor.pages is 3
pentap panel previous
wait 100
pentap panel previous
wait 100
expect editor.page is 1
# the pen window: contact-level touch after the pen waits it out
wait 1100
expect touch.gesture is
expect display.presents not clear
swipe 2 930 900 0 400
expect editor.modal is pages
expect touch.gesture is 2.down
expect display.presents is full 0 0 1860 2480
swipe 2 930 1400 0 -400
expect editor.modal is none
expect touch.gesture is 2.up
# left turns to the next page, right to the previous
swipe 2 1300 1200 -400 0
expect touch.gesture is 2.left
expect editor.page is 2
swipe 2 600 1200 400 0
expect touch.gesture is 2.right
expect editor.page is 1
# past the ends nothing is added: the toast says where it stopped
swipe 2 1300 1200 -400 0
swipe 2 1300 1200 -400 0
expect editor.page is 3
swipe 2 1300 1200 -400 0
expect toast is This is the last page.
expect editor.pages is 3
expect editor.page is 3
wait 1700
swipe 2 600 1200 400 0
swipe 2 600 1200 400 0
expect editor.page is 1
swipe 2 600 1200 400 0
expect toast is This is the first page.
expect editor.pages is 3
expect editor.page is 1
# the panel's + still adds a page, on the last one
pentap panel next
wait 100
pentap panel next
wait 100
pentap panel next
wait 100
expect editor.pages is 4
expect gestures.pages is touch.2.down
