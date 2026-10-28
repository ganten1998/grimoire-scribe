# settings: status_mode=1
# rust-only
# The pen wins. A swipe a little after the pen left, and 60 mm from where it wrote, fires; with the pen
# hovering, a swipe is refused ("pen") and says so once in a while.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pen line 300 700 500 720 20 0.6 0.6
pen leave
wait 350
swipe 2 1300 1500 0 400
expect touch.gesture is 2.down
expect editor.modal is pages
expect touch.reject is
swipe 2 1300 1500 0 -400
expect editor.modal is none
expect toast is
# hovering: refused, and the toast
pen hover 900 1200
swipe 2 1100 1200 0 400
expect touch.reject is pen
expect editor.modal is none
expect toast is Move the pen away, then swipe.
# again at once: no second toast
wait 2100
expect toast is
swipe 2 1100 1200 0 400
expect touch.reject is pen
expect editor.modal is none
expect toast is
