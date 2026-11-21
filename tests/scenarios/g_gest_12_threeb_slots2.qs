# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# env: GRIMOIRE_SIM_TOUCH_SLOTS=2
# copy: tests/fixtures/gestures/three .
wait 200
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 200
expect screen is editor
tapn 3 930 1100
expect touch.reject is three
expect editor.modal is none
tap panel library
wait 100
tap library settings
wait 100
expect library.ui_at.329.830 is settings.cat.4
tap 329 830
wait 100
expect library.ui_at.1233.639 is gestures
tap 1233 639
wait 100
expect library.ui_at.930.920 is gestures.group.pages
tap 930 920
wait 100
expect library.gestures.7 is Pages: no gesture
