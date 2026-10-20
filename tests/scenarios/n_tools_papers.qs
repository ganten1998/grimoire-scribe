# Tool picker + new pens, sheet music notebook
wait 200
tap 965 95
wait 100
shot 01-new-sheet-papers
expect library.sheet is new
tap 1066 893
wait 50
expect library.paper is 3
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 3
pentap 1768 358
wait 200
shot 02-tool-picker
expect editor.modal is tools
pentap 872 727
wait 200
expect editor.modal is none
expect editor.tool is fountain
pen curve 300 600 40 70 0.7 4
pentap 1768 358
pentap 1504 561
expect editor.tool is marker
pen curve 300 900 40 70 0.7 4
pentap 1768 358
pentap 1188 561
expect editor.tool is fineliner
pen curve 300 1200 40 70 0.7 4
wait 700
shot 03-pens-on-music
expect editor.strokes is 3
# Custom music from the editor's paper sheet: start from the last layout, add a bass staff
# (after the first-run calibration tip has gone: this shot is in the README)
wait 5000
expect toast is
pentap 1768 1242
wait 200
expect editor.modal is paper
pentap 1622 1452
wait 200
shot 04-builder
expect editor.modal is music
expect editor.music is 5: treble tab6
pentap 579 1279
expect editor.music is 4: treble tab6 bass
pentap 1571 1943
wait 500
shot 05-custom-page
expect editor.modal is none
expect editor.paper is custom
expect editor.music is 4: treble tab6 bass
expect editor.strokes is 3
quit
