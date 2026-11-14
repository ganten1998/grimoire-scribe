# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/penbutton/single .
# env: GRIMOIRE_SIM_CLOCK=script
# One click of the side button, with a triple click bound beside it: the click acts (Undo) only once the
# 350 ms in which a second click may follow its release are over. App time is the script's own
# (GRIMOIRE_SIM_CLOCK=script, docs/SIM.md): the click moves it on by its 80 ms press, and each wait by its
# own length and no more. So the check that Undo has NOT run comes 200 ms after the release, and the check
# that it has 400 ms after it, however late the machine ends a wait (by the real clock a late first wait
# ended after Undo had run).
wait 200
expect gestures.undo is pen.Button.tap
tap 965 95
wait 100
expect library.sheet is new
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.strokes is 0
pen line 300 700 1300 700 20 0.6 0.6
expect editor.strokes is 1
side click 1
expect pen.clicks is 1
expect editor.strokes is 1
wait 200
expect editor.strokes is 1
wait 200
expect pen.gesture is Button.tap
expect pen.clicks is 0
expect editor.strokes is 0
