# Rename a notebook with accents through the keyboard's accents page (Gothic), then again in
# Classic. Keys (portrait): shift 175 2205, del 1679 2205, Cancel 217 2365, the page key 696 2365
# (labelled with a and e acute on the letters page, abc on the accents page), space 1060 2365,
# Done 1643 2365. Accents page rows: y 1725 a e i o u acute then grave, x = 133 + 177 n; y 1885
# circumflex then diaeresis, same x; y 2045 n tilde, c cedilla, a o tilde, sharp s, oe, ae,
# o slash, a ring, x = 221 + 177 n; y 2205 inverted ? and !, angle quotes, middle dot,
# x = 570 + 177 n. The notebook's card opens at 326 608, its menu at 547 1052 (Rename 930 842).
# The checks read the keyboard's text, so they hold the accents themselves (UTF-8).
# rust-only
wait 200
tap 965 95
wait 100
tap 790 460
tap 1564 2254
wait 300
pen line 300 900 1300 950 80 0.2 0.9
pentap panel library
# Let the mirror PDF be written (the page has ink), so each rename moves it to the new name:
# grimoire/PDFs/<name>.pdf ends up with the accented name.
wait 1500
expect settings.mirror_pdf is 1
# The card's menu, Rename; clear the default name ("Notebook Oct 19", at most 15 characters).
tap 547 1052
wait 200
tap 930 842
wait 200
expect library.sheet is keyboard
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
expect library.text is
# "Musica espanola" with its u acute and n tilde: shift, m, the accents page, u acute, back,
# "sica espa", accents, n tilde, back, "ola".
tap 175 2205
tap 1455 2205
tap 696 2365
tap 841 1725
tap 696 2365
expect library.text is Mú
tap 398 2045
tap 1372 1885
tap 747 2205
tap 221 2045
tap 1060 2365
tap 487 1885
tap 398 2045
tap 1726 1885
tap 221 2045
tap 696 2365
wait 100
shot 01-accents-page-gothic
tap 221 2045
tap 696 2365
tap 1549 1885
tap 1637 2045
tap 221 2045
wait 100
expect library.text is Música española
shot 02-typed-gothic
tap 1643 2365
wait 300
expect library.sheet is none
shot 03-renamed-gothic
# The renamed notebook opens (the keyboard is gone, so the tap reaches the card), and back.
tap 326 608
wait 300
expect screen is editor
pentap panel library
wait 300
expect screen is library
# The card's menu shows the name in the blackletter title face.
tap 547 1052
wait 200
expect library.sheet is more
shot 04-menu-title-gothic
tap 930 1475
wait 200
expect library.sheet is none
# Classic: Settings, Style, Done.
tap 1535 95
wait 200
tap 496 937
wait 300
expect settings.theme is classic
tap 1557 1930
wait 300
expect library.sheet is none
# Rename again: the keyboard opens on the name the Library stored, accents whole. Add a space,
# an inverted exclamation mark, "Ol" (O with shift) and e acute.
tap 547 1052
wait 200
tap 930 842
wait 200
expect library.text is Música española
tap 1060 2365
tap 696 2365
wait 100
shot 05-accents-page-classic
tap 747 2205
tap 696 2365
tap 175 2205
tap 1549 1885
tap 1637 2045
tap 696 2365
tap 175 2205
wait 100
shot 06-accents-page-classic-shift
tap 175 2205
tap 310 1725
wait 100
expect library.text is Música española ¡Olé
shot 07-typed-classic
tap 1643 2365
wait 300
expect library.sheet is none
shot 08-renamed-classic
# Once more to read the stored name back, then Cancel; the notebook still opens.
tap 547 1052
wait 200
tap 930 842
wait 200
expect library.text is Música española ¡Olé
tap 217 2365
wait 300
expect library.sheet is none
tap 326 608
wait 300
expect screen is editor
pentap panel library
wait 1500
expect screen is library
quit
