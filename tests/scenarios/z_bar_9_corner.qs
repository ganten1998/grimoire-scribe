# rust-only
# The clock corner's tap is the clock's alone with the Links bar up, while a page is looked at, and over a link
# drawn there (which is not followed); a tap outside the bar that is not the corner then stays on the page above;
# a tap on the panel's Links button is the button again, which cancels to the page the bar began on.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
expect editor.page is 1
key 38 0
pen line 40 40 300 300 30 0.6 0.6
expect editor.picking is new
tap 450 95
expect editor.pages_tab is pages
tap 703 340
expect editor.links is 4 4
expect editor.page is 1
pen leave
wait 1100
# the link over the corner: the corner's tap is the clock's, the link not followed
tap 60 60
expect settings.status_on is 1
expect editor.page is 1
tap 60 60
expect settings.status_on is 0
# with the bar up
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 2
expect editor.bar.1 is p2 x2
expect editor.bar.2 is p3 x2
tap 60 60
expect settings.status_on is 1
expect editor.bar is open
tap 60 60
expect settings.status_on is 0
# while a page is looked at
tap 479 2218
wait 300
expect editor.bar is peek
expect editor.bar.peek is 2
tap 60 60
expect settings.status_on is 1
expect editor.bar is peek
expect editor.page is 1
tap 60 60
expect settings.status_on is 0
# a tap outside it that is not the corner stays on the page above
tap 600 1100
wait 300
expect editor.bar is 0
expect editor.page is 2
key 48 0
wait 300
expect editor.page is 1
# the panel's Links button again: back to the origin, not a stay
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
expect editor.bar is peek
tap panel links
wait 300
expect editor.bar is 0
expect editor.page is 1
