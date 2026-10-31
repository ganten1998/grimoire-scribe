# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures/three .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
tapn 3 930 1100
expect touch.gesture is 3.tap
expect editor.modal is pages
