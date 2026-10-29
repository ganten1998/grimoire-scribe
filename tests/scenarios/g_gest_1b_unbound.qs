# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
# settings: status_mode=1
# rust-only
# With two fingers up unbound (gestures.ini: touch.2.up=nothing) the swipe does nothing, the panel's Links
# button starts a link directly, a bound Links key still opens the bar, and two fingers down close it: the bar's
# cancel is fixed, whatever is bound (without it 2-down would be Pages, which swaps the menu and keeps what
# the bar was looking at).
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures-unbound .
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
wait 1100
expect gestures.links is
swipe 2 930 1400 0 -400
wait 200
expect editor.bar is 0
expect editor.modal is none
expect touch.gesture is 2.up
pentap panel links
expect editor.bar is 0
expect editor.link_mode is 1
pentap panel links
expect editor.link_mode is 0
key 37 0
wait 400
expect editor.bar is open
swipe 2 930 900 0 400
wait 200
expect editor.bar is 0
expect editor.modal is none
expect editor.page is 1
expect touch.gesture is 2.down
