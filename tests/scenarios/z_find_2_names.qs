# rust-only
# Names: letters find the pages the Index names, accents and capitals ignored, best first (the name begins with
# them, then a word does, then anywhere inside) and in page order among equals; the rows say the name alone;
# no name that matches is a line that is not a button.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
key 34 0
wait 300
expect editor.find is go
tap 221 2045
tap 664 1885
tap 1101 2205
expect editor.text is arb
expect editor.find.rows is 1
expect editor.find.1 is Árbol
expect editor.find.2 is
shot 01-find-arb
# two letters: both begin so, in page order (Arbol is page 3, Arena page 5)
tap 1679 2205
expect editor.text is ar
expect editor.find.rows is 2
expect editor.find.1 is Árbol
expect editor.find.2 is Arena
shot 02-find-ar
# a letter that neither has
tap 570 2205
expect editor.text is arx
expect editor.find.rows is 1
expect editor.find.1 is No page names match.
# a name found inside: "ren" is in Arena only, "bol" in Arbol only
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 664 1885
tap 487 1885
tap 1278 2205
expect editor.text is ren
expect editor.find.rows is 1
expect editor.find.1 is Arena
# a row that is a name goes there
tap 930 1390
wait 300
expect editor.find is 0
expect editor.page is 5
expect editor.back is 1 1
# nothing typed, nothing shown
key 34 0
wait 300
expect editor.find.rows is 0
expect editor.find.1 is
