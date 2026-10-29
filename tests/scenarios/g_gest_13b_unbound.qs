# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
fpath 1300 1100 800 1100 400
expect touch.gesture is
expect editor.modal is none
expect fshape.trail is
expect fshape.fired is 0
