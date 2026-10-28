# settings: status_mode=1
# The pen's button clears the page only from the page itself. Held 1.2 s (or clicked three times)
# while a sheet is in front of it (the tool picker, in Pin mode too, the Display sheet), or while
# the pen taps the panel's buttons, it clears nothing, also once the sheet is closed with the
# button still down. Held over the bare page it still clears it (undoable). (side_button=3: the
# hand that draws white keeps a finger on the button.)
# settings: mirror_pdf=0
# settings: side_button=3
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
pen line 300 1000 1500 1000 60 0.7 0.7
wait 700
expect editor.strokes is 1
# Held over the tool picker
pentap panel tool-picker
wait 200
expect editor.modal is tools
side on
wait 1500
side off
wait 200
expect editor.modal is tools
expect editor.strokes is 1
expect toast not Page cleared
# Clicked three times over it
side on
side off
side on
side off
side on
side off
wait 400
expect editor.strokes is 1
expect toast not Page cleared
# Held while Pin mode pins the highlighter, then Done and the picker closed, the button still down
pentap 1600 273
wait 200
expect editor.pin_mode is 1
side on
pentap 1504 727
wait 1300
expect editor.pins is highlighter
expect editor.strokes is 1
pentap 1600 273
wait 200
pentap 300 1800
wait 300
expect editor.modal is none
wait 1200
side off
wait 200
expect editor.strokes is 1
expect toast not Page cleared
# Held while the pen taps the highlighter's pin, and on after it
side on
pentap panel tool:highlighter
wait 1500
side off
wait 200
expect editor.tool is highlighter
expect editor.strokes is 1
expect toast not Page cleared
# Held over the Display sheet (under the pin now)
pentap panel display
wait 200
expect editor.modal is display
side on
wait 1500
side off
wait 200
expect editor.modal is display
expect editor.strokes is 1
expect toast not Page cleared
tap 1462 1860
wait 200
expect editor.modal is none
# Over the bare page: the hold clears it, and undo brings it back
side on
wait 1500
side off
wait 200
expect toast is Page cleared
expect editor.strokes is 0
pentap panel undo
wait 300
expect editor.strokes is 1
pentap panel library
wait 500
expect screen is library
quit
