# make l10n-check runs this in every language, in Classic with each font of Settings > Font (Terminess,
# Atkinson Hyperlegible Next, Noto Sans, Literata), with GRIMOIRE_CHECK_FIT=1: Settings with the Style
# row's Font chip, then the Font sheet with each family highlighted (its sample drawn in it), upright and
# sideways, and Cancel, which leaves the font as it was. (2.1, Rust only)
wait 100
expect settings.theme is classic
tap 1535 95
wait 50
expect library.sheet is settings
expect library.ui_has.font is 1
shot 01-settings
rotate 1
wait 100
shot 02-settings-sideways
tap library font
wait 50
expect library.sheet is font
shot 03-font-sideways
tap library font.terminess
wait 50
tap library font.atkinson
wait 50
expect library.font.pick is atkinson
shot 04-font-atkinson-sideways
tap library font.noto-sans
wait 50
shot 05-font-noto-sans-sideways
tap library font.literata
wait 50
shot 06-font-literata-sideways
rotate 0
wait 100
expect library.sheet is font
shot 07-font-literata
tap library font.noto-sans
wait 50
shot 08-font-noto-sans
tap library font.atkinson
wait 50
shot 09-font-atkinson
tap library font.terminess
wait 50
expect library.font.pick is terminess
shot 10-font-terminess
tap library font.cancel
wait 50
expect library.sheet is settings
quit
