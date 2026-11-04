# settings: status_mode=1
# rust-only
# An owner updating from the build before (a settings.ini at version 3 with no gestures key): the first time
# the page comes up sideways it is whole, and nothing is said (the sideways hint, hints bit 8, went in
# 2.0.0-beta.2 with the owner's toast pass), then or at the next turn.
# (The first-open tip, hints bit 0, is g_hint_1_first_open's: it is already seen here.)
# settings: settings_version=3
# settings: whatsnew_hide=1
# settings: hints=1
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect settings.hints is 1
rotate 1
wait 1200
expect editor.zoom is 1.00
expect editor.scale is 0.75
expect toast is
expect settings.hints is 1
rotate 0
wait 800
rotate 1
wait 1200
expect toast is
expect settings.hints is 1
