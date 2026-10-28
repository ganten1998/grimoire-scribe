# settings: status_mode=1
# Library -> new medium-rule notebook -> write with all three tools -> erase/undo/redo -> pages -> panel
# rust-only
wait 300
shot 01-library-empty
expect screen is library
expect library.items is 0
tap 965 95
wait 200
shot 02-new-sheet
expect library.sheet is new
tap 510 460
wait 100
expect library.paper is 1
tap 1564 2254
wait 400
shot 03-editor-medium
expect screen is editor
expect editor.paper is 1
expect editor.page is 1
expect editor.pages is 1
expect editor.tool is pencil
expect editor.strokes is 0
# Pencil (default tool): handwriting-like loops at rising pressure
pen curve 300 700 40 80 0.35 4
pen curve 300 840 40 80 0.7 4
# Pencil shading with tilt
pen line 300 1000 1200 1000 120 0.5 0.5 55 0
pen line 300 1040 1200 1040 120 0.5 0.5 55 0
pen line 300 1080 1200 1080 120 0.5 0.5 55 0
wait 100
expect editor.strokes is 5
# Mechanical pencil
pentap panel tool-picker
expect editor.modal is tools
pentap 1188 395
wait 100
expect editor.modal is none
expect editor.tool is mech
pen curve 300 1300 35 80 0.6 5
pen line 300 1420 1400 1420 100 0.1 1.0
# Ballpoint
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.tool is ballpoint
pen curve 300 1650 35 80 0.6 5
pen line 300 1770 1400 1770 100 0.1 1.0
wait 300
shot 04-three-tools
expect editor.strokes is 9
# Erase the ballpoint line with the eraser end (one gesture)
eraser line 800 1700 820 1850 20
wait 300
shot 05-erased
expect editor.strokes is 8
expect editor.tool is ballpoint
pentap panel undo
wait 300
shot 06-undo-erase
expect editor.strokes is 9
pentap panel redo
wait 300
shot 07-redo-erase
expect editor.strokes is 8
pentap panel undo
wait 200
expect editor.strokes is 9
# New page, draw, back
pentap panel next
wait 400
expect editor.page is 2
expect editor.pages is 2
expect editor.strokes is 0
pen curve 400 600 50 60 0.6 3
wait 200
shot 08-page2
expect editor.strokes is 1
pentap panel previous
wait 400
shot 09-back-page1
expect editor.page is 1
expect editor.pages is 2
expect editor.strokes is 9
# Collapse, drag the box, expand
pentap panel collapse
wait 300
shot 10-collapsed
expect editor.panel is collapsed
drag touch 1782 300 300 2200 8
wait 300
shot 11-collapsed-moved
expect editor.panel is collapsed
expect editor.strokes is 9
tap panel expand
wait 300
shot 12-expanded-moved
expect editor.panel is expanded
expect editor.modal is none
wait 1800
quit
