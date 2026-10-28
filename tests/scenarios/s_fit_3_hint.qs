# rust-only
# An owner updating from the build before (a settings.ini at version 3 with no gestures key) is told once,
# the first time the page comes up sideways whole, why two fingers swipe there now; a fresh install is not.
# settings: settings_version=3
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect settings.hints is 0
rotate 1
wait 1200
expect toast is Sideways now shows the whole page, so two-finger swipes work. Pinch to zoom in; Display changes this.
expect settings.hints is 256
wait 6200
expect toast is
rotate 0
wait 800
rotate 1
wait 1200
expect toast is
expect settings.hints is 256
