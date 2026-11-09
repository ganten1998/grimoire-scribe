# settings: status_mode=1
# root: y_fonts_own_1_sheet
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# The font of the owner's own that y_fonts_own_1_sheet picked is the font when Grimoire starts again: loaded
# before the first screen is drawn (its bytes hash to what the check passed), and the crash guard's mark is
# gone once that screen is on the panel. Every screen of the Library is drawn in it. (2.1.0-beta.2)
# rust-only
wait 100
expect settings.theme is classic
expect settings.font is file:GrimoireTest-Regular.ttf
expect font is file:GrimoireTest-Regular.ttf
expect font.own.drawn is 1
expect font.guard is
shot 01-library-own
tap 1535 95
wait 50
expect library.sheet is settings
shot 02-settings-own
tap library font
wait 50
expect library.font.pick is file:GrimoireTest-Regular.ttf
expect library.font.checking is 0
tap library font.cancel
wait 50
expect font is file:GrimoireTest-Regular.ttf
tap library done
wait 50
# New notebook's sheet and the keyboard in it
tap 960 95
wait 50
shot 03-new-own
quit
