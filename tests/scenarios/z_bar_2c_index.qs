# settings: status_mode=1
# rust-only
# An entry for the Index: a tap on it leaves the bar for the Pages view's Index tab (there is no page to look at).
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/index-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
wait 100
pentap panel previous
wait 100
key 38 0
pen line 300 1000 900 1200 30 0.6 0.6
expect editor.picking is new
tap 1250 95
expect editor.link.1 is index
expect editor.links is 1 1
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 1
expect editor.bar.1 is index
shot 01-bar-with-the-index
tap 479 2218
wait 300
expect editor.bar is 0
expect editor.modal is pages
expect editor.pages_tab is index
