# rust-only
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
wait 100
expect settings.theme is gothic
tap 1535 95
wait 50
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 50
shot es-gestures-0
rotate 1
wait 100
shot es-gestures-0-sideways
rotate 0
wait 100
expect library.ui_at.930.749 is gestures.group.editing
tap 930 749
wait 50
shot es-editing-0
rotate 1
wait 100
shot es-editing-0-sideways
rotate 0
wait 100
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 50
swipe 2 930 1100 0 400 10 295 300
expect library.record.line is Apoya los dedos a la vez.
shot es-0-Put
rotate 1
wait 100
shot es-0-Put-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 50
swipe 2 930 1100 400 400
expect library.record.line is Desliza en línea recta.
shot es-0-Swipe
rotate 1
wait 100
shot es-0-Swipe-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 50
tapn 1 930 1100
expect library.record.line is Un dedo es para tocar y enlaces.
shot es-0-One
rotate 1
wait 100
shot es-0-One-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 50
expect library.ui_at.1360.1650 is record.cancel
tap 1360 1650
wait 50
expect library.ui_at.930.1616 is done
tap 930 1616
wait 50
expect library.ui_at.930.2069 is done
tap 930 2069
wait 50
tap 930 930
wait 100
expect settings.theme is classic
expect library.ui_at.1550.1930 is done
tap 1550 1930
wait 50
tap 1535 95
wait 50
expect library.ui_at.890.1930 is gestures
tap 890 1930
wait 50
shot es-gestures-1
rotate 1
wait 100
shot es-gestures-1-sideways
rotate 0
wait 100
expect library.ui_at.930.749 is gestures.group.editing
tap 930 749
wait 50
shot es-editing-1
rotate 1
wait 100
shot es-editing-1-sideways
rotate 0
wait 100
expect library.ui_at.930.656 is gestures.row.1
tap 930 656
wait 50
swipe 2 930 1100 0 400 10 295 300
expect library.record.line is Apoya los dedos a la vez.
shot es-1-Put
rotate 1
wait 100
shot es-1-Put-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 50
swipe 2 930 1100 400 400
expect library.record.line is Desliza en línea recta.
shot es-1-Swipe
rotate 1
wait 100
shot es-1-Swipe-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 50
tapn 1 930 1100
expect library.record.line is Un dedo es para tocar y enlaces.
shot es-1-One
rotate 1
wait 100
shot es-1-One-sideways
rotate 0
wait 100
expect library.ui_at.930.1650 is record.retry
tap 930 1650
wait 50
expect library.ui_at.1360.1650 is record.cancel
tap 1360 1650
wait 50
expect library.ui_at.930.1616 is done
tap 930 1616
wait 50
expect library.ui_at.930.2069 is done
tap 930 2069
wait 50
expect library.ui_at.1550.1930 is done
tap 1550 1930
wait 50
