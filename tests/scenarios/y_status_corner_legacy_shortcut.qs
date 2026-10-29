# rust-only
# settings: shortcuts_enabled=1
# copy: tests/fixtures/status-bar-keys .
# env: TZ=UTC
wait 500
clock 1700000000
battery 37
hid attach pad Pro Controller
tap 965 95
wait 100
tap 1564 2254
wait 500
expect status.reserved is 0
expect status.shown is 1
expect settings.status_mode is 1
# Retired status-mode bindings toggle the corner; they cannot resurrect the bar.
key 50 0
wait 300
expect settings.status_mode is 1
expect settings.status_on is 0
expect status.reserved is 0
expect status.shown is 0
key 50 0
wait 300
expect settings.status_mode is 1
expect settings.status_on is 1
expect status.shown is 1
tap 20 20
wait 300
expect status.shown is 0
key 50 0
wait 300
expect status.shown is 1
shot 01-corner-shortcut
pentap panel library
wait 300
expect status.shown is 0
quit
