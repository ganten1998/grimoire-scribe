# Charcoal and the calligraphy pen on medium rule. Tilt is in the page frame: (40, 46) is a
# right hand leaning the pen toward the bottom right; (-40, 46) a left hand; (0, 0) upright.
wait 200
tap 965 95
wait 100
tap 510 460
wait 50
expect library.paper is 1
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 1
# Charcoal: a pressure ramp, a loop, then the stick laid low on its side (tilt 70).
pentap 1768 358
pentap 1504 395
expect editor.tool is charcoal
pen line 150 330 1300 330 60 0.05 1.0 30 30
pen curve 200 520 45 90 0.85 4 30 30
pen line 150 700 1300 700 60 0.7 0.7 50 50
wait 100
expect editor.strokes is 3
# Calligraphy, right-handed lean: zigzags show thick down-right, hairline up-right.
pentap 1768 358
pentap 1188 727
expect editor.tool is calligraphy
pen line 150 1000 330 1180 30 0.7 0.7 40 46
pen line 330 1180 510 1000 30 0.7 0.7 40 46
pen line 510 1000 690 1180 30 0.7 0.7 40 46
pen line 690 1180 870 1000 30 0.7 0.7 40 46
pen curve 1000 1090 70 120 0.7 3 40 46
# Left-handed lean: the edge mirrors.
pen line 150 1400 330 1580 30 0.7 0.7 -40 46
pen line 330 1580 510 1400 30 0.7 0.7 -40 46
pen line 510 1400 690 1580 30 0.7 0.7 -40 46
pen curve 1000 1490 70 120 0.7 3 -40 46
# Upright pen: the default 45 degree nib.
pen line 150 1800 330 1980 30 0.7 0.7 0 0
pen line 330 1980 510 1800 30 0.7 0.7 0 0
pen curve 1000 1890 70 120 0.7 3 0 0
# Let the ink settle before the README shot.
wait 300
expect toast is
shot 01-charcoal-calligraphy
expect editor.strokes is 15
pentap 1768 358
wait 200
shot 02-picker
expect editor.modal is tools
quit
