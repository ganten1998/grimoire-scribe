# settings: status_mode=1
# rust-only
# An owner who updated and whose first notebook comes up sideways gets the first-open tip (bit 0) at once:
# there is no sideways hint to wait for since 2.0.0-beta.2 (the owner's toast pass), and bit 8 stays clear.
# The next open says nothing.
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
expect toast is New: two fingers down for the Graph, up for page links.
expect settings.hints is 1
wait 6300
expect toast is
pentap panel library
wait 500
expect screen is library
tap 200 400
wait 600
expect screen is editor
expect toast is
expect settings.hints is 1
