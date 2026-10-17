# Tool picker + new pens, grand staff notebook
wait 200
tap 965 95
wait 100
shot 01-new-sheet-papers
tap 1066 893
wait 50
tap 1564 2254
wait 400
pentap 1768 358
wait 200
shot 02-tool-picker
pentap 872 727
wait 200
pen curve 300 600 40 70 0.7 4
pentap 1768 358
pentap 1504 561
pen curve 300 900 40 70 0.7 4
pentap 1768 358
pentap 1188 561
pen curve 300 1200 40 70 0.7 4
wait 700
shot 03-pens-on-music
# Custom music from the editor's paper sheet: start from the last layout, add a bass staff
# (after the first-run calibration tip has gone: this shot is in the README)
wait 5000
pentap 1768 1242
wait 200
pentap 1622 1452
wait 200
shot 04-builder
pentap 579 1279
pentap 1571 1943
wait 500
shot 05-custom-page
quit
