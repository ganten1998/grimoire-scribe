# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/pages-many/notebooks notebooks
# rust-only
# The Pages view turns its screen by one finger flicked sideways (left: the next screen), by the page-turn
# keys and a controller's buttons, as the notebook's page does (2.0.0-rc.12: its arrows left the bottom
# row). A slow drag, or one mostly up and down, turns nothing.
wait 200
tap 200 400
wait 200
expect editor.pages is 120
pentap panel pages
wait 200
expect editor.modal is pages
expect editor.pages.screen is 1
pen leave
wait 1100
fpath 1400 1300 600 1320 200
wait 200
expect editor.pages.screen is 2
fpath 1400 1300 600 1320 200
wait 200
expect editor.pages.screen is 3
fpath 400 1300 1200 1280 200
wait 200
expect editor.pages.screen is 2
fpath 1400 1300 1320 1310 200
wait 200
expect editor.pages.screen is 2
fpath 1000 900 1150 1500 200
wait 200
expect editor.pages.screen is 2
expect editor.modal is pages
key 106 0
wait 200
expect editor.pages.screen is 3
key 105 0
wait 200
expect editor.pages.screen is 2
shot 01-screen-2
quit
