# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# env: GRIMOIRE_SIM_TOUCH_SLOTS=2
# copy: tests/fixtures/gestures/three .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
tapn 3 930 1100
expect touch.reject is three
expect editor.modal is none
tap panel library
wait 100
tap 1535 95
wait 100
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 100
expect library.ui_at.930.1069 is gestures.group.pages
tap 930 1069
wait 100
expect library.gestures.7 is Pages: none
