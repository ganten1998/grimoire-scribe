# The keyboard in Classic: its letters, symbols and accents pages upright and sideways, and a
# name with symbols typed (u_keyboard_symbols.qs types the same in Gothic)
# rust-only
# settings: theme=classic
wait 200
expect settings.theme is classic
tap 1155 95
wait 100
expect library.sheet is keyboard
shot 01-letters
tap 175 2205
tap 1283 2045
tap 1549 1885
tap 487 1885
tap 1289 2365
tap 398 2045
expect library.text is Joe's
tap 1018 2365
tap 1278 2205
tap 1549 1885
tap 841 1885
tap 487 1885
tap 398 2045
tap 1018 2365
# The symbols key: digits, then ! @ # $ % ^ & * ( ), and so on
tap 570 2365
wait 100
shot 02-symbols
tap 1549 1885
tap 310 1725
tap 1726 1885
tap 133 1885
expect library.text is Joe's notes (2)!
# Its accents key goes to the accents page, and abc there back to the letters
tap 747 2365
wait 100
shot 03-accents
tap 747 2365
wait 100
tap 1466 2365
tap 393 2365
expect library.text is Joe's notes (2)!.-
tap 1679 2205
tap 1679 2205
expect library.text is Joe's notes (2)!
tap 1685 2365
wait 300
expect library.sheet is none
expect library.folders is Joe's notes (2)!
# A folder name may not hold a slash (the store refuses it, and says so)
tap 1155 95
wait 100
tap 221 2045
tap 570 2365
tap 1549 2045
tap 570 2365
tap 1101 2205
expect library.text is a/b
tap 1685 2365
wait 300
expect toast has without a slash
expect library.folders is Joe's notes (2)!
# Sideways: the same three pages; ' at its place there
rotate 1
wait 300
tap library new-folder
wait 100
expect library.sheet is keyboard
tap 1723 1745
expect library.text is '
shot 04-letters-sideways
tap 756 1745
wait 100
shot 05-symbols-sideways
tap 995 1745
wait 100
shot 06-accents-sideways
tap 221 1745
wait 100
expect library.sheet is none
quit
