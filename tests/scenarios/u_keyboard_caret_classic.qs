# The keyboard's caret, a selection and the edit page in Classic, upright and sideways (u_keyboard_caret.qs
# checks them in Gothic): the caret between two letters is a bar in the space before the second, a selection is
# white on black, the edit key is drawn pressed on the edit page.
# rust-only
# settings: theme=classic
clock 1700000000
wait 3600
expect settings.theme is classic
tap 1155 95
wait 100
expect library.sheet is keyboard
tap 398 2045
tap 1106 2045
tap 487 1885
tap 487 1885
tap 841 1885
tap 1549 1570
tap 1549 1570
expect library.caret is she|et
wait 1200
shot 01-caret-inside
drag touch 120 1570 1100 1570 4
expect library.caret is [sheet|]
wait 1200
shot 02-selected
tap 1372 1570
wait 100
shot 03-edit-page
tap 1372 2177
expect library.caret is sheet|
rotate 1
wait 300
shot 04-edit-sideways
tap 2076 950
expect library.caret is shee|t
tap 1837 950
wait 1200
shot 05-letters-sideways
tap 221 1745
wait 100
expect library.sheet is none
quit
