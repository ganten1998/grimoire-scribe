# settings: status_mode=1
# rust-only
# A peek is gone when the Kindle sleeps (the bar closes, the page it began on is there on waking) and when
# Grimoire is left: notebook.ini was never written with the peeked page.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
expect editor.bar is peek
sleep
expect asleep is 1
wake
wait 400
expect asleep is 0
expect editor.bar is 0
expect editor.page is 1
expect editor.bar.peek is 0
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
expect editor.bar is peek
powerhold
expect running is 0
