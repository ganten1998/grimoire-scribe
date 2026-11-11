# The caret and the edit page in Spanish (2.1.0-beta.2, issue #36), with every string's fit checked: ñ typed at
# the caret, the edit page's words (Seleccionar todo, Cortar, Copiar, Pegar, Al inicio, Al final; borrar on its
# wider delete), upright and sideways, in Gothic.
# rust-only
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
clock 1700000000
wait 3600
expect language is es
tap 1155 95
wait 100
expect library.sheet is keyboard
# "nio", back two, ñ (the letters page's last key in the middle row in Spanish, 1726 2045), delete the n
tap 1278 2205
tap 1372 1885
tap 1549 1885
tap 1549 1570
tap 1549 1570
expect library.caret is n|io
tap 1726 2045
tap 1549 1570
tap 1679 2205
expect library.caret is |ñio
wait 1200
shot 01-caret
tap 1372 1570
wait 100
shot 02-edit-page
tap 1372 2177
expect library.caret is ñio|
rotate 1
wait 300
shot 03-edit-sideways
tap 221 1745
wait 100
expect library.sheet is none
quit
