# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/gestures/actions .
# Startup cached light 10 and warmth 6; unknown-cache refusal is covered by test_actions.
key 7 0
expect toast is Light 11 of 24
key 8 0
expect toast is Light 12 of 24
key 9 0
expect toast is Warmth 7 of 24
key 10 0
expect toast is Warmth 8 of 24
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
key 11 0
expect settings.rotation_lock is 1
rotate 1
wait 250
expect rotation is 0
key 11 0
expect settings.rotation_lock is 0
key 16 0
expect settings.pinch_zoom is 0
key 16 0
expect settings.pinch_zoom is 1
expect editor.panel_rect is 1704 220 128 1518
expect display.presents not clear
key 17 0
expect settings.theme is classic
expect display.presents is full 0 0 1860 2480
key 17 0
expect settings.theme is gothic
key 18 0
expect settings.side_button is 3
key 19 0
expect settings.mirror_pdf is 1
key 19 0
expect settings.mirror_pdf is 0
key 20 0
expect settings.sleep_mode is 1
expect display.presents not clear
key 6 0
expect settings.invert is 1
expect display.presents is full 0 0 1860 2480
key 3 0
expect editor.zoom not 1.00
key 2 0
expect editor.zoom is 1.00
pentap panel next
pentap panel previous
key 3 0
key 3 0
key 3 0
expect editor.zoom is 1.95
key 5 0
expect editor.page is 2
expect editor.zoom is 1.95
key 4 0
expect editor.page is 1
expect editor.zoom is 1.95
