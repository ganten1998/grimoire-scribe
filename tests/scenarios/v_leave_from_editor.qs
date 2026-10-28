# settings: status_mode=1
# env: GRIMOIRE_BG_DELAY_MS=3000
# pdfs: 2
# Leaving straight from the editor (the 4 s hold) while another notebook's PDF is still being
# written: the open notebook's PDF is written too, by the child that finishes the exports after
# Grimoire has gone. (The first export is slowed down as a big notebook's is on the Kindle, so the
# second one queues behind it.)
# rust-only
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen line 300 900 1300 950 40 0.6 0.6
pentap panel library
wait 400
expect screen is library
expect library.items is 1
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.strokes is 0
pen line 300 1200 1300 1250 40 0.6 0.6
wait 100
power down
power held
wait 300
expect running is 0
