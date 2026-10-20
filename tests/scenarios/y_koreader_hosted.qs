# env: GRIMOIRE_HOST=koreader
# pdfs: 1
# Started from KOReader's Grimoire plugin (kindle/app/grimoire.sh --hosted koreader sets
# GRIMOIRE_HOST): the Library's Exit reads Back to KOReader, in a button as wide as the words, in
# Gothic and Classic, upright and sideways (01, 02, 04, 05). Sleep mode 2, the Kindle's own
# screensaver, needs the Kindle UI that KOReader keeps paused, so the page shows instead (03: the
# page alone, no toolbar). Back to KOReader leaves the way Exit does.
wait 200
expect host is koreader
shot 01-library-back-to-koreader
rotate 1
wait 300
shot 02-library-sideways
rotate 0
wait 300
# A notebook with one stroke; its Display sheet, the sleep screen tapped round to the Kindle's own.
tap 833 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen line 300 1700 1400 1720 60 0.4 0.9
wait 700
expect editor.strokes is 1
pentap 1768 1350
wait 300
expect editor.modal is display
tap 800 1172
wait 200
tap 800 1172
wait 200
expect settings.sleep_mode is 2
tap 1464 1628
wait 300
expect editor.modal is none
sleep
wait 400
shot 03-asleep-mode-2-shows-the-page
expect asleep is 1
wake
wait 400
expect asleep is 0
expect screen is editor
# The Library in Classic: Settings, Style, Done.
pentap 1768 1560
wait 300
expect screen is library
tap 1403 95
wait 200
tap 496 937
wait 300
expect settings.theme is classic
tap 1557 1930
wait 300
expect library.sheet is none
shot 04-library-classic
rotate 1
wait 300
shot 05-library-classic-sideways
rotate 0
wait 300
tap 1658 95
expect running is 0
