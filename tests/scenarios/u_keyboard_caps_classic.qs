# The shift key's three states in Classic, upright and sideways: shift once the pressed key, caps on the same
# key with a bar under its label, off again (u_keyboard_caps.qs types with them in Gothic)
# rust-only
# settings: theme=classic
wait 200
expect settings.theme is classic
tap 1155 95
wait 100
expect library.sheet is keyboard
tap 175 2205
wait 100
shot 01-shift-once
tap 175 2205
wait 100
shot 02-caps-on
# O, K, the space bar, 7: caps on stays
tap 1549 1885
tap 1460 2045
tap 1018 2365
tap 1195 1725
tap 747 2365
wait 100
shot 03-caps-on-accents
tap 747 2365
tap 1549 1885
expect library.text is OK 7O
tap 175 2205
wait 100
shot 04-off-again
tap 1549 1885
expect library.text is OK 7Oo
tap 175 2365
wait 300
expect library.sheet is none
rotate 1
wait 300
tap library new-folder
wait 100
expect library.sheet is keyboard
tap 221 1585
wait 100
shot 05-shift-once-sideways
tap 221 1585
wait 100
shot 06-caps-on-sideways
tap 283 1425
tap 1473 1585
expect library.text is AB
tap 221 1745
wait 100
expect library.sheet is none
quit
