# rust-only
# settings: shortcuts_enabled=1
# copy: tests/fixtures/status-bar-keys .
# env: TZ=UTC
wait 500
clock 1700000000
battery 37
# Controller indicator responds to devices that are available to Grimoire.
hid attach pad Pro Controller
tap 965 95
wait 100
tap 1564 2254
wait 500
expect status.reserved is 104
expect status.controller is 1
expect status.live is 0
expect status.fill is 4
shot 01-controller
# Cycling the mode refits without changing the page's ink.
key 50 0
wait 300
expect settings.status_mode is 1
expect status.reserved is 0
expect status.shown is 0
tap 20 20
wait 300
expect status.shown is 1
key 50 0
wait 300
expect settings.status_mode is 2
expect status.shown is 0
key 50 0
wait 300
expect settings.status_mode is 0
expect status.reserved is 104
expect status.shown is 1
# The bar is not on the Library; Live view's indicator joins it upon returning.
pentap panel library
wait 300
expect status.shown is 0
tap 1345 95
wait 300
tap 318 2100
wait 300
tap 1538 2100
wait 300
expect screen is library
expect live.on is 1
tap 500 460
wait 500
expect screen is editor
expect status.live is 1
shot 02-sharing
quit
