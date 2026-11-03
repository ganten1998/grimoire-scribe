# rust-only
# generate: tests/map_fixture.py --library 3x3
# Two fingers spreading over the Graph opened from the Library zoom it (through touchgest, as on the
# Kindle), as they do over the editor's Graph: the Library's touch view lets a spread be a zoom while the
# Graph is up. Before, the Library's touch view left zoom_ok off, so touchgest never classed a spread as a
# zoom: one finger still and the other spreading away on a diagonal was refused as a crooked swipe and the
# map did not move.
wait 300
swipe 2 930 900 0 400
expect library.map is graph
wait 2500
expect library.map.zoom is 3.11
finger 1 down 900 1000
finger 2 down 1100 1200
frame 20
finger 1 move 900 1000
finger 2 move 1150 1250
frame 160
finger 1 move 900 1000
finger 2 move 1250 1350
frame 160
finger 1 move 900 1000
finger 2 move 1350 1450
frame 160
finger 1 move 900 1000
finger 2 move 1450 1550
frame 160
finger 1 move 900 1000
finger 2 move 1500 1600
frame 160
finger 1 up 900 1000
frame 10
finger 2 up 1500 1600
frame
expect library.map is graph
expect library.map.zoom is 6.00
