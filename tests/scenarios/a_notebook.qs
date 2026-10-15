# Library -> new medium-rule notebook -> write with all three tools -> erase/undo/redo -> pages -> panel
wait 300
shot 01-library-empty
tap 965 95
wait 200
shot 02-new-sheet
tap 510 460
wait 100
tap 1564 2254
wait 400
shot 03-editor-medium
# Pencil (default tool): handwriting-like loops at rising pressure
pen curve 300 700 40 80 0.35 4
pen curve 300 840 40 80 0.7 4
# Pencil shading with tilt
pen line 300 1000 1200 1000 120 0.5 0.5 55 0
pen line 300 1040 1200 1040 120 0.5 0.5 55 0
pen line 300 1080 1200 1080 120 0.5 0.5 55 0
wait 100
# Mechanical pencil
pentap 1768 358
pentap 1504 395
wait 100
pen curve 300 1300 35 80 0.6 5
pen line 300 1420 1400 1420 100 0.1 1.0
# Ballpoint
pentap 1768 358
pentap 1188 561
wait 100
pen curve 300 1650 35 80 0.6 5
pen line 300 1770 1400 1770 100 0.1 1.0
wait 300
shot 04-three-tools
# Erase the ballpoint line with the eraser end (one gesture)
eraser line 800 1700 820 1850 20
wait 300
shot 05-erased
pentap 1768 594
wait 300
shot 06-undo-erase
pentap 1768 702
wait 300
shot 07-redo-erase
pentap 1768 594
wait 200
# New page, draw, back
pentap 1768 1114
wait 400
pen curve 400 600 50 60 0.6 3
wait 200
shot 08-page2
pentap 1768 938
wait 400
shot 09-back-page1
# Collapse, drag the box, expand
pentap 1795 260
wait 300
shot 10-collapsed
drag touch 1782 300 300 2200 8
wait 300
shot 11-collapsed-moved
tap 300 2200
wait 300
shot 12-expanded-moved
wait 1800
quit
