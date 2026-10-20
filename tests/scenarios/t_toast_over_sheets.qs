# env: GRIMOIRE_PASSCODE=left-off
# A toast that is up when a sheet opens (here the notice a start shows for 10 s) moves above the
# sheet's buttons, or above the keyboard, so their labels stay readable; it goes back to its
# usual place when the sheet closes, and the same holds sideways.
wait 300
expect toast has passcode was left off
# (x y w h: its usual place, near the bottom)
expect toast.rect is 112 2250 1636 110
tap 965 95
wait 100
expect library.sheet is new
# above Cancel and Create (their tops at 2180), in the space under "In: Library"
expect toast.rect is 112 2050 1636 110
shot 01-new-sheet-toast-above-its-buttons
tap 300 2250
wait 100
expect library.sheet is none
expect toast.rect is 112 2250 1636 110
tap 1155 95
wait 100
expect library.sheet is keyboard
# above the keyboard (its top at 1480)
expect toast.rect is 112 1350 1636 110
shot 02-keyboard-toast-above-it
tap 217 2365
wait 100
expect library.sheet is none
rotate 1
wait 400
expect rotation is 1
expect toast.rect is 422 1630 1636 110
tap 1585 95
wait 100
expect library.sheet is new
expect toast.rect is 422 1496 1636 110
shot 03-new-sheet-landscape-toast-above-its-buttons
# Custom music over it: the toast clears the music sheet's own controls
tap 1240 1116
wait 200
expect library.sheet is music
expect toast.rect is 422 895 1636 110
shot 04-music-landscape-toast-clear-of-its-controls
tap 100 900
wait 200
expect library.sheet is new
tap 570 1700
wait 100
expect library.sheet is none
tap 2155 95
wait 100
expect library.sheet is settings
expect toast.rect is 422 1496 1636 110
shot 05-settings-landscape-toast-above-done
quit
