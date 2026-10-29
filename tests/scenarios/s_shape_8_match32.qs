# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures/match32 .
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect gestures.shape_count is 32
expect gestures.refused is 0
pen line 300 650 1300 670 10 0.6 0.6
pen leave
wait 1100
shape Z 930 1150 45 600
expect editor.strokes is 0
expect fshape.fired is 1
expect fshape.last has fired undo
expect main.longest_unpumped_ms is ok
