# settings: status_mode=1
# rust-only
# Sideways with the page filling the width (landscape_fit off): the view rests at the top, so an upward
# drag pans (a downward one would pan nowhere), and a drag to the left turns the page.
# settings: landscape_fit=0
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
expect editor.zoom is 1.00
expect editor.zoomed is 0
expect editor.scrolls is 1 0
swipe 2 1240 1200 0 -400
expect editor.modal is none
expect touch.gesture is
expect editor.zoomed is 1
expect editor.zoom is 1.00
expect editor.page is 1
swipe 2 1500 900 -400 0
expect touch.gesture is 2.left
expect editor.page is 2
