# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/gestures/stress8 .
# SYNTHETIC ONLY: owner J/K and older device drags were not supplied.
# Python ports of fingershape_synth palm drift stand in; no owner acceptance is claimed.
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect gestures.shape_count is 8
trace tests/fixtures/gestures/synthetic-drift-1.txt
expect fshape.fired is 0
expect editor.strokes is 0
trace tests/fixtures/gestures/synthetic-drift-2.txt
expect fshape.fired is 0
expect editor.strokes is 0
trace tests/fixtures/gestures/synthetic-drift-3.txt
expect fshape.fired is 0
expect editor.strokes is 0
trace tests/fixtures/gestures/synthetic-drift-4.txt
expect fshape.fired is 0
expect editor.strokes is 0
expect main.longest_unpumped_ms is ok
