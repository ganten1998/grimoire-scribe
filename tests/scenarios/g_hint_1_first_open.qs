# settings: status_mode=1
# rust-only
# An owner updating from the build before (a settings.ini at version 3 with no gestures key) is told
# once, as the first notebook comes up, what two fingers up and down do now (hints bit 0): one toast
# for six seconds, set down as seen, and not said again at the next open.
# settings: settings_version=3
# settings: whatsnew_hide=1
# settings: mirror_pdf=0
wait 300
expect settings.hints is 0
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect toast is New: two fingers up show this page's links. Two fingers down open the Graph.
expect settings.hints is 1
wait 3000
expect toast is New: two fingers up show this page's links. Two fingers down open the Graph.
wait 3300
expect toast is
pentap panel library
wait 500
expect screen is library
wait 3000
expect toast is
tap 200 400
wait 400
expect screen is editor
expect toast is
expect settings.hints is 1
