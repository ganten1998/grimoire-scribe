# settings: status_mode=1
# rust-only
# Asleep, a gesture is dropped (the screen is the Kindle's); awake again, the editor is as it was.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
sleep
wait 300
swipe 2 930 900 0 400
expect asleep is 1
wake
wait 400
expect asleep is 0
expect editor.modal is none
