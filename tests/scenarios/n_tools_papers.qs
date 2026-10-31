# settings: status_mode=1
# Tool picker + new pens, sheet music notebook
# rust-only
wait 200
tap 965 95
wait 100
shot 01-new-sheet-papers
expect library.sheet is new
tap library paper.3
wait 50
expect library.paper is 3
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 3
pentap panel tool-picker
wait 200
shot 02-tool-picker
expect editor.modal is tools
pentap 872 727
wait 200
expect editor.modal is none
expect editor.tool is fountain
pen curve 300 600 40 70 0.7 4
pentap panel tool-picker
pentap 1504 561
expect editor.tool is marker
pen curve 300 900 40 70 0.7 4
pentap panel tool-picker
pentap 1188 561
expect editor.tool is fineliner
pen curve 300 1200 40 70 0.7 4
wait 700
shot 03-pens-on-music
expect editor.strokes is 3
# Custom music from the editor's paper sheet: start from the last layout, add a bass staff
# Let the ink settle before the README shot.
wait 300
expect toast is
pentap panel paper
wait 200
expect editor.modal is paper
pentap panel paper.custom
wait 200
shot 04-builder
expect editor.modal is music
expect editor.music is 3: treble bass tab6
pentap panel music.add.bass
expect editor.music is 3: treble bass tab6 bass
pentap panel music.done
wait 500
shot 05-custom-page
expect editor.modal is none
expect editor.paper is custom
expect editor.music is 3: treble bass tab6 bass
expect editor.strokes is 3
quit
