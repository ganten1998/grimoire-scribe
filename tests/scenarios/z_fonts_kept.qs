# settings: status_mode=1
# root: y_fonts
# The font y_fonts picked (Literata) is the font when Grimoire starts again: saved by the tap itself (y_fonts
# went Back and quit with the storage full after it), and applied before the first screen is drawn. (2.1)
# rust-only
wait 100
expect settings.theme is classic
expect settings.font is literata
expect font is literata
expect settings.kept.font is
shot 01-library-literata
tap 1535 95
wait 50
expect library.sheet is settings
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.font.pick is literata
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is literata
quit
