# settings: status_mode=1
# rust-only
# One finger flicked sideways across the page turns it, as two fingers do: left = next, right =
# previous. A slow or short one-finger stroke does not.
# settings: mirror_pdf=0
# settings: gesture_new_page=0
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pentap panel next
wait 100
pentap panel next
wait 100
pentap panel previous
wait 100
pentap panel previous
wait 100
expect editor.page is 1
wait 1100
fpath 1400 1200 500 1220 250
wait 400
expect editor.page is 2
fpath 1400 1200 500 1220 250
wait 400
expect editor.page is 3
fpath 500 1200 1400 1180 250
wait 400
expect editor.page is 2
# too slow: a drawn line, not a flick
fpath 1400 1200 500 1220 1500
wait 400
expect editor.page is 2
# too short
fpath 1000 1200 900 1200 150
wait 400
expect editor.page is 2
# too steep
fpath 1000 900 700 1500 250
wait 400
expect editor.page is 2
