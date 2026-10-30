# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
# settings: status_mode=1
# rust-only
# Zoomed in, two fingers pan the page: the zoom held, a drag that starts at the top edge pans too, and a
# drag at a scroll limit still pans (nowhere): it never navigates. The first pan says once where Pages
# and Links went. A surface that does not pan (the Pages view) makes the same drag a swipe, which the
# mirror rule closes it with.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.scrolls is 0 0
expect settings.hints is 0
pinch 930 1200 300 900 6
wait 400
expect editor.zoomed is 1
expect editor.zoom is 2.25
expect editor.scrolls is 1 1
# a vertical drag pans, holds the zoom, opens nothing, and the pan hint shows
swipe 2 930 1200 0 400
expect editor.modal is none
expect touch.gesture is
expect editor.zoom is 2.25
expect editor.zoomed is 1
expect toast is Here two fingers move the page. Zoom out for page and link swipes.
expect settings.hints is 64
wait 4200
expect toast is
# starting 40 px from the top edge
swipe 2 930 40 0 400
expect editor.modal is none
expect touch.gesture is
expect editor.zoom is 2.25
expect toast is
# a drag at the scroll limit (the top of the page by now) pans nowhere and opens nothing
swipe 2 930 1200 0 400
swipe 2 930 1200 0 400
swipe 2 930 1200 0 400
expect editor.modal is none
expect touch.gesture is
expect editor.zoom is 2.25
expect editor.zoomed is 1
# left and right pan too while the page scrolls sideways
swipe 2 1300 1200 -400 0
expect editor.page is 1
expect editor.zoom is 2.25
expect touch.gesture is
# the Pages view does not pan: the same drag is a swipe, and two fingers up close it
pentap panel pages
tap panel pages.graph
wait 200
expect editor.modal is map
expect editor.scrolls is 0 0
swipe 2 930 1400 0 -400
expect editor.modal is none
expect touch.gesture is 2.up
expect editor.zoomed is 1
expect editor.zoom is 2.25
# the Links bar does not pan either: with it up two fingers up or down are swipes, and two fingers down close
# it back to the page as it was left, zoom and all
key 37 0
wait 400
expect editor.bar is open
expect editor.scrolls is 0 0
swipe 2 930 900 0 400
expect editor.bar is 0
expect editor.modal is none
expect touch.gesture is 2.down
expect editor.zoomed is 1
expect editor.zoom is 2.25
