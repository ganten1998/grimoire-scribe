# rust-only
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
wait 100
expect settings.theme is gothic
tap 1535 95
wait 50
tap library gestures
wait 50
shot es-gestures-0
rotate 1
wait 100
shot es-gestures-0-sideways
rotate 0
wait 100
tap library gestures.group.editing
wait 50
shot es-editing-0
rotate 1
wait 100
shot es-editing-0-sideways
rotate 0
wait 100
tap library gestures.row.1
wait 50
swipe 2 930 1100 0 400 10 295 300
expect library.record.line is Apoya los dedos a la vez.
shot es-0-Put
rotate 1
wait 100
shot es-0-Put-sideways
rotate 0
wait 100
tap library record.retry
wait 50
swipe 2 930 1100 400 400
expect library.record.line is Desliza en línea recta.
shot es-0-Swipe
rotate 1
wait 100
shot es-0-Swipe-sideways
rotate 0
wait 100
tap library record.retry
wait 50
tapn 1 930 1100
expect library.record.line is Un dedo es para tocar y enlaces.
shot es-0-One
rotate 1
wait 100
shot es-0-One-sideways
rotate 0
wait 100
tap library record.retry
wait 50
tap library record.cancel
wait 50
tap library done
wait 50
tap library done
wait 50
tap 930 930
wait 100
expect settings.theme is classic
tap library done
wait 50
tap 1535 95
wait 50
tap library gestures
wait 50
shot es-gestures-1
rotate 1
wait 100
shot es-gestures-1-sideways
rotate 0
wait 100
tap library gestures.group.editing
wait 50
shot es-editing-1
rotate 1
wait 100
shot es-editing-1-sideways
rotate 0
wait 100
tap library gestures.row.1
wait 50
swipe 2 930 1100 0 400 10 295 300
expect library.record.line is Apoya los dedos a la vez.
shot es-1-Put
rotate 1
wait 100
shot es-1-Put-sideways
rotate 0
wait 100
tap library record.retry
wait 50
swipe 2 930 1100 400 400
expect library.record.line is Desliza en línea recta.
shot es-1-Swipe
rotate 1
wait 100
shot es-1-Swipe-sideways
rotate 0
wait 100
tap library record.retry
wait 50
tapn 1 930 1100
expect library.record.line is Un dedo es para tocar y enlaces.
shot es-1-One
rotate 1
wait 100
shot es-1-One-sideways
rotate 0
wait 100
tap library record.retry
wait 50
tap library record.cancel
wait 50
tap library done
wait 50
tap library done
wait 50
tap library done
wait 50
