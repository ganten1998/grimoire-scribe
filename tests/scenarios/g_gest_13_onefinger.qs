# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/gestures/onefinger .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
fpath 1300 1100 800 1100 400
expect touch.gesture is
expect editor.modal is map
expect fshape.fired is 1
fpath 1300 1100 800 1100 400
expect editor.modal is map
expect fshape.trail is
expect fshape.fired is 0
key 45 0
expect editor.modal is none
fpath 1768 1000 1268 1000 400
expect editor.modal is none
expect fshape.fired is 0
key 45 0
pentap panel tool-picker
wait 1100
expect editor.modal is tools
fpath 1300 1100 800 1100 400
expect editor.modal not map
expect fshape.fired is 0
expect fshape.trail is
