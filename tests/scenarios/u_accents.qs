# Rename a notebook with accents through the keyboard's accents page (Gothic), then again in
# Classic. Keys (portrait): shift 175 2205, del 1679 2205, the page key 696 2365 (labelled with
# a and e acute on the letters page, abc on the accents page), space 1060 2365, Done 1643 2365.
# Accents page rows: y 1725 a e i o u acute then grave, x = 133 + 177 n; y 1885 circumflex then
# diaeresis, same x; y 2045 n tilde, c cedilla, a o tilde, sharp s, oe, ae, o slash, a ring,
# x = 221 + 177 n; y 2205 inverted ? and !, angle quotes, x = 658 + 177 n.
wait 200
tap 965 95
wait 100
tap 790 460
tap 1564 2254
wait 300
pen line 300 900 1300 950 80 0.2 0.9
pentap 1768 1566
# Let the mirror PDF be written (the page has ink), so each rename moves it to the new name:
# grimoire/PDFs/<name>.pdf ends up with the accented name.
wait 1500
# The card's menu, Rename; clear the default name ("Notebook Oct 19", at most 15 characters).
tap 547 1052
wait 200
tap 930 842
wait 200
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
# "Musica espanola" with its u acute and n tilde: shift, m, the accents page, u acute, back,
# "sica espa", accents, n tilde, back, "ola".
tap 175 2205
tap 1455 2205
tap 696 2365
tap 841 1725
tap 696 2365
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
shot 02-typed-gothic
tap 1643 2365
wait 300
expect screen is library
shot 03-renamed-gothic
# The card's menu shows the name in the blackletter title face.
tap 547 1052
wait 200
shot 04-menu-title-gothic
tap 930 1475
wait 200
# Classic: Settings, Style, Done.
tap 1535 95
wait 200
tap 496 937
wait 300
tap 1557 1930
wait 300
expect screen is library
# Rename again: add a space, an inverted exclamation mark, "Ol" (O with shift) and e acute.
tap 547 1052
wait 200
tap 930 842
wait 200
tap 1060 2365
tap 696 2365
wait 100
shot 05-accents-page-classic
tap 835 2205
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
shot 07-typed-classic
tap 1643 2365
wait 300
expect screen is library
shot 08-renamed-classic
quit
