# rust-only
# Sideways the whole page shows (landscape_fit, on by default): at rest the page does not scroll on either
# axis, so two fingers swipe anywhere, and pinching zooms in from the whole page.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
wait 100
pentap panel previous
wait 100
rotate 1
wait 1200
expect screen is editor
expect size is 2480x1860
expect editor.zoom is 1.00
expect editor.scale is 0.75
expect editor.zoomed is 0
expect editor.scrolls is 0 0
swipe 2 1240 700 0 400
expect editor.modal is pages
expect touch.gesture is 2.down
swipe 2 1240 1300 0 -400
expect editor.modal is none
swipe 2 1500 900 -400 0
expect touch.gesture is 2.left
expect editor.page is 2
swipe 2 700 900 400 0
expect touch.gesture is 2.right
expect editor.page is 1
# pinching in from the whole page zooms, and the page then scrolls on both axes
pinch 1240 930 300 900 6
wait 400
expect editor.zoomed is 1
expect editor.scrolls is 1 1
