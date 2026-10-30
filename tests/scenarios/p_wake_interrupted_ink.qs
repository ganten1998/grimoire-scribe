# rust-only
# settings: mirror_pdf=0
# A missed sleep announcement must still end a held stroke on the real wake. Its delayed
# release must not join the next stroke or leave handwriting mode active under menus.
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pen down 400 900 0.6
pen move 650 920 0.6
expect display.pen_mode is 1
wake
wait 200
expect asleep is 0
expect display.pen_mode is 0
expect editor.strokes is 1
pen up 650 920
expect editor.strokes is 1
expect editor.modal is none
pen line 400 1200 900 1200 30 0.6 0.6
expect editor.strokes is 2
sleep
wait 100
expect asleep is 1
expect display.pen_mode is 0
wake
wait 200
expect editor.strokes is 2
expect editor.modal is none
expect editor.screen_composed is 1
quit
