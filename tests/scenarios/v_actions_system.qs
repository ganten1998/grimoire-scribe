# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/gestures/actions .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect language is en
key 25 0
expect language not en
expect settings.language not auto
key 30 0
expect settings.sleep_minutes is 15
key 31 0
expect settings.updates is 1
# Gestures also remains assignable to a key while shortcuts are enabled.
key 22 0
expect settings.gestures is 0
key 22 0
expect settings.gestures is 1
# A touch binding can disable shortcuts; another touch enables them again.
tapn 2 930 1100
expect settings.shortcuts_enabled is 0
tapn 2 930 1100
expect settings.shortcuts_enabled is 1
tapn 3 930 1100
expect settings.controller is 1
tapn 3 930 1100
expect settings.controller is 0
# The recovery message and the master row keep gestures recoverable.
swipe 2 700 1100 400 0
expect settings.gestures is 0
expect toast is Gestos desactivados. Ajustes > Gestos los vuelve a activar.
swipe 2 930 1100 0 400
expect editor.modal is none
key 21 0
wait 100
expect asleep is 1
wake
wait 200
tap panel library
wait 200
tap 1535 95
wait 100
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 100
expect library.ui_at.930.420 is gestures.master
tap 930 420
expect settings.gestures is 1
