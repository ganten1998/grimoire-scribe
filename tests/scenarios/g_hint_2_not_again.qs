# rust-only
# The settings file the update left is this build's now (it holds a gestures key), so the gate is shut
# even where the bit is clear again: no tip at the first open of a started program. The sideways hint,
# the other tip about changed behaviour, is quiet as well.
# root: g_hint_1_first_open
# settings: hints=0
wait 300
expect screen is library
expect settings.hints is 0
tap 200 400
wait 400
expect screen is editor
expect toast is
expect settings.hints is 0
rotate 1
wait 1200
expect toast is
expect settings.hints is 0
