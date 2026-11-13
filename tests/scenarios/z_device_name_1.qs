# rust-only
# settings: status_mode=1
# settings: shortcuts_enabled=1
# env: GRIMOIRE_SIM_DEVICE_NAME=Slate GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/device-name-keys .
# A platform whose device has a name of its own (sys_device_name; i18n.rs, the device's name): a text
# that says Kindle says that name. The rotation lock's two toasts (one names the device), then Settings,
# whose "Turn with the ..." row is in the shot.
wait 300
key 19 0
expect toast is Rotation locked.
wait 100
key 19 0
expect toast is Rotation follows Slate.
tap library settings
wait 300
expect library.sheet is settings
shot 01-settings-on-a-slate
quit
