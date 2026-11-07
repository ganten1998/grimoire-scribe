# settings: status_mode=1
# settings: theme=classic
# settings: font=comic-sans
# A font= this version does not have (a newer version's, or one written by hand): Classic draws in
# Terminess, and settings.ini keeps the line as it was while the font is not picked, also when another
# setting is saved; Cancel in the Font sheet keeps it too. Picking a font and Done replaces it. (2.1)
# rust-only
wait 100
expect settings.font is terminess
expect font is terminess
expect settings.kept.font is comic-sans
tap 1535 95
wait 50
expect library.sheet is settings
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
tap library font.noto-sans
wait 50
tap library font.cancel
wait 50
expect font is terminess
expect settings.kept.font is comic-sans
# Another setting saved: the line stays
tap 930 1060
wait 50
tap 930 1060
wait 50
expect settings.mirror_pdf is 1
expect settings.kept.font is comic-sans
# Done on Terminess, the font in use: nothing changes, the line stays
tap library font
wait 50
tap library font.done
wait 50
expect settings.kept.font is comic-sans
# A pick and Done: the font, written as it is
tap library font
wait 50
tap library font.noto-sans
wait 50
tap library font.done
wait 100
expect font is noto-sans
expect settings.font is noto-sans
expect settings.kept.font is
shot 01-settings-noto-sans
quit
