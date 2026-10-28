# rust-only
# A keyboard in front (the Pages view's naming keyboard): a swipe is refused ("keyboard"), whatever it is
# bound to, and nothing changes.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/link-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
key 49 0
wait 300
expect editor.modal is pages
expect editor.keyboard is Page name
swipe 2 930 900 0 400
expect editor.keyboard is Page name
expect editor.modal is pages
expect touch.reject is keyboard
expect touch.gesture is
