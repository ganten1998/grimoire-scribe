# rust-only
# settings: status_mode=1
# settings: shortcuts_enabled=1
# env: GRIMOIRE_SIM_DEVICE_NAME=Slate GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/device-name-keys .
# The same in Spanish: the translation keeps the word Kindle, and the device's name takes its place.
wait 300
key 19 0
wait 100
key 19 0
expect toast is La rotación sigue al Slate.
quit
