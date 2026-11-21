# settings: status_mode=1
# settings: theme=classic
# settings: font=comic-sans
# A font= this version does not have (a newer version's, or one written by hand): Classic draws in
# Terminess, and settings.ini keeps the line as it was while no font is picked, also when another
# setting is saved; the Font sheet opened and left (its Back, or its X) keeps it too. A font tapped
# replaces it at once: applied, and written as it is (the kept line is gone only once settings.ini is
# written with the new font); the sheet's Back keeps that font. (2.1)
# rust-only
wait 100
expect settings.font is terminess
expect font is terminess
expect settings.kept.font is comic-sans
tap library settings
wait 50
expect library.sheet is settings
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
# Back with no font tapped: nothing changes, the line stays
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is terminess
expect settings.kept.font is comic-sans
# Another setting saved: the line stays
tap library settings.cat.6
wait 50
tap library mirror
wait 50
tap library mirror
wait 50
expect settings.mirror_pdf is 1
expect settings.kept.font is comic-sans
# The X with no font tapped (Terminess, the font in use): nothing changes, the line stays
tap library settings.cat.1
wait 50
tap library font
wait 50
tap library font.done
wait 50
expect library.sheet is none
expect settings.kept.font is comic-sans
# A font tapped: applied and saved at once, the font written as it is
tap library settings
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
tap library font.noto-sans
wait 100
expect font is noto-sans
expect settings.font is noto-sans
expect settings.kept.font is
# Back keeps it
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is noto-sans
expect settings.font is noto-sans
expect settings.kept.font is
shot 01-settings-noto-sans
quit
