# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# A fresh hover sample is valid for 50 ms. Keep this contact burst inside that window.
# The nearer contact starts 952 px (80.6 mm at 300 dpi) from the pen.
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
pen hover 100 1200
swipe 2 1200 1200 0 400 1 295 0
expect touch.gesture is 2.down
expect editor.modal is map
pen leave
wait 1100
swipe 2 1200 1500 0 -400
expect editor.modal is none
# Here the nearer contact starts 352 px (29.8 mm) from the fresh hover sample.
pen hover 700 1200
swipe 2 1200 1200 0 400 1 295 0
expect touch.reject is pen
expect editor.modal is none
expect toast is Move the pen away, then swipe.
wait 2100
expect toast is
pen hover 700 1200
swipe 2 1200 1200 0 400 1 295 0
expect touch.reject is pen
expect toast is
pen leave
