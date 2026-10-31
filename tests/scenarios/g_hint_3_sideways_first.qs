# settings: status_mode=1
# rust-only
# Two tips never meet. An owner who updated and whose first notebook comes up sideways is told why the page
# is whole (the sideways hint, bit 8) and the first-open tip (bit 0) waits for the next open, instead of
# replacing it at once and leaving bit 8 saved as seen.
# settings: settings_version=3
# settings: whatsnew_hide=1
# settings: mirror_pdf=0
wait 300
expect settings.hints is 0
rotate 1
wait 800
tap 1583 93
wait 300
tap 1910 1696
wait 1200
expect screen is editor
expect toast is Sideways now shows the whole page, so two-finger swipes work. Pinch to zoom in; Display changes this.
expect settings.hints is 256
wait 6300
expect toast is
pentap panel library
wait 500
expect screen is library
tap 200 400
wait 600
expect screen is editor
expect toast is New: two fingers up show this page's links. Two fingers down open the Graph.
expect settings.hints is 257
wait 6300
expect toast is
