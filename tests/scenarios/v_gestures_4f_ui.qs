# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# env: GRIMOIRE_CHECK_FIT=1 GRIMOIRE_SIM_PEN_BUTTON2=1
# copy: tests/fixtures/penbutton/gallery .
# Language-neutral gallery: validated in EN/ES and Gothic/Classic.
# The wait state plus a nonempty line asserts a refusal without depending on its translation.
wait 200
tap 1535 95
wait 100
expect library.ui_at.890.1930 is gestures
pentap 890 1930
wait 100
shot p6-gestures
rotate 1
wait 100
shot p6-gestures-sideways
rotate 0
wait 100
expect library.ui_at.930.749 is gestures.group.editing
pentap 930 749
wait 100
shot p6-editing
rotate 1
wait 100
shot p6-editing-sideways
rotate 0
wait 100
expect library.ui_at.930.920 is gestures.row.4
tap 930 920
wait 100
expect library.gestures.learning is hold-eraser
expect library.record is wait
shot p6-card-body
rotate 1
wait 100
shot p6-card-body-sideways
rotate 0
wait 100
pen hover 930 1100
side click 1
wait 400
expect library.record is wait
expect library.record.line not
shot p6-hold-eraser-refusal
rotate 1
wait 100
shot p6-hold-eraser-refusal-sideways
rotate 0
wait 100
expect library.ui_at.1360.1650 is record.cancel
pentap 1360 1650
wait 100
expect library.ui_at.930.440 is gestures.row.1
tap 930 440
wait 100
expect library.gestures.learning is undo
expect library.record is wait
side2 click 2
wait 400
expect library.record is pattern pen.Button2.double
expect library.record.line not
shot p6-second-button-double
rotate 1
wait 100
shot p6-second-button-double-sideways
rotate 0
wait 100
expect library.ui_at.1360.1650 is record.cancel
pentap 1360 1650
wait 100
expect library.ui_at.930.2030 is done
pentap 930 2030
wait 100
expect library.ui_at.930.1229 is gestures.group.index
pentap 930 1229
wait 100
shot p6-index
rotate 1
wait 100
shot p6-index-sideways
rotate 0
wait 100
expect library.ui_at.930.520 is gestures.row.1
tap 930 520
wait 100
expect library.gestures.learning is links
expect library.record is wait
side click 2
wait 400
expect library.record is pattern pen.Button.double
expect library.record.line not
shot p6-button-double
rotate 1
wait 100
shot p6-button-double-sideways
rotate 0
wait 100
expect library.ui_at.1360.1650 is record.cancel
pentap 1360 1650
wait 100
expect library.ui_at.930.1790 is done
pentap 930 1790
wait 100
expect library.ui_at.930.1069 is gestures.group.pages
pentap 930 1069
wait 100
shot p6-pages
rotate 1
wait 100
shot p6-pages-sideways
rotate 0
wait 100
expect library.ui_at.930.1800 is gestures.row.10
tap 930 1800
wait 100
expect library.gestures.learning is pan
expect library.record is wait
side click 1
wait 400
expect library.record is wait
expect library.record.line not
shot p6-pan-click-refusal
rotate 1
wait 100
shot p6-pan-click-refusal-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
side on
wait 1300
side off
expect library.record is wait
expect library.record.line not
shot p6-pan-hold-refusal
rotate 1
wait 100
shot p6-pan-hold-refusal-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
side on
pen down 800 1100 0.6
pen move 1000 1200 0.6
expect display.ink is 0
pen up 1000 1200
side off
expect library.record is pattern pen.Button.drag
expect library.record.line not
shot p6-pan-drag
rotate 1
wait 100
shot p6-pan-drag-sideways
rotate 0
wait 100
expect library.ui_at.1360.1650 is record.cancel
pentap 1360 1650
wait 100
expect library.ui_at.930.1960 is gestures.row.11
tap 930 1960
wait 100
expect library.gestures.learning is zoom
expect library.record is wait
side click 1
wait 400
expect library.record is wait
expect library.record.line not
shot p6-zoom-click-refusal
rotate 1
wait 100
shot p6-zoom-click-refusal-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
side on
wait 1300
side off
expect library.record is wait
expect library.record.line not
shot p6-zoom-hold-refusal
rotate 1
wait 100
shot p6-zoom-hold-refusal-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
pentap 930 1650
wait 100
side on
pen down 800 1100 0.6
pen move 1000 1200 0.6
expect display.ink is 0
pen up 1000 1200
side off
expect library.record is pattern pen.Button.drag
expect library.record.line not
shot p6-zoom-drag
rotate 1
wait 100
shot p6-zoom-drag-sideways
rotate 0
wait 100
expect library.ui_at.1360.1650 is record.cancel
pentap 1360 1650
wait 100
