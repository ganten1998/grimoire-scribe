# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# copy: tests/fixtures/penbutton/button2 .
# env: GRIMOIRE_SIM_PEN_BUTTON2=1
wait 200
expect pen.gesture is
tap library settings
wait 100
expect library.ui_at.329.830 is settings.cat.4
tap 329 830
wait 100
expect library.ui_at.1233.639 is gestures
tap 1233 639
wait 100
expect library.sheet is gestures
expect library.ui_at.930.608 is gestures.group.editing
tap 930 608
wait 100
expect library.gestures_group is editing
expect library.gestures.1 is Undo: second pen button double-click
expect library.gestures.2 is Redo: no gesture
expect library.ui_at.930.450 is gestures.row.2
tap 930 450
wait 100
expect library.gestures.learning is redo
expect library.record is wait
pen hover 930 1100
side click 2
wait 400
expect library.record is pattern pen.Button.double
expect library.ui_at.500.1650 is record.use
pentap 500 1650
wait 100
expect library.record is 0
pen leave
expect library.ui_at.125.95 is done
tap 125 95
wait 100
expect library.ui_at.125.95 is settings.back
tap 125 95
wait 100
expect library.ui_at.1735.95 is done
tap 1735 95
wait 100
expect library.sheet is none
tap library add
tap library add.new-notebook
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
side2 click 2
wait 400
expect pen.gesture is Button2.double
expect editor.strokes is 0
