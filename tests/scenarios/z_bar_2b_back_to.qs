# settings: status_mode=1
# rust-only
# The back list in the bar: after a page was stayed on from the bar, the bar on the new page offers "Back to"
# the page it came from, which looks at it like any entry; staying there pops the list (as a browser's Back
# does) instead of pushing the page again.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
tap 1381 2218
wait 300
tap 600 1100
wait 300
expect editor.page is 2
expect editor.back is 1 1
# page 2 has no links: its bar is empty, with the way back
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 0
tap 160 2046
wait 300
expect editor.bar is peek
expect editor.bar.peek is 1
expect editor.page is 2
shot 01-back-to-page-1
tap 600 1100
wait 300
expect editor.bar is 0
expect editor.page is 1
expect editor.back is 0 0
