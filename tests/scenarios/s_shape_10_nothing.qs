# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# root: s_shape_1_record
# A finger shape whose action does nothing fires with nothing said (2.0.0-beta.2, the owner: "Show the action's
# name only when the action actually did something"): the recorded Z, Undo, on a blank page with nothing to undo.
# After a stroke the same Z undoes it and says Undo, as before; drawn again, with nothing left to undo, nothing.
# (the clock corner held still)
clock 1700000000
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect editor.strokes is 0
shape Z 930 1150 45 400
expect fshape.fired is 1
expect fshape.last has fired undo
expect toast is
pen line 350 650 1250 680 12 0.6 0.6
pen leave
wait 1100
expect editor.strokes is 1
shape Z 930 1150 45 400
expect fshape.fired is 1
expect editor.strokes is 0
expect toast is Undo
wait 1300
expect toast is
shape Z 930 1150 45 400
expect fshape.fired is 1
expect fshape.last has fired undo
expect editor.strokes is 0
expect toast is
quit
