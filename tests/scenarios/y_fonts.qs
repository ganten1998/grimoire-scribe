# settings: status_mode=1
# settings: theme=classic
# settings: shortcuts_enabled=1
# env: GRIMOIRE_CHECK_FIT=1
# Settings > Font (2.1, Classic only): the Style row's chip opens the Font sheet with the font in use
# highlighted; a tap on a family highlights it and changes nothing else; Cancel leaves the font as it
# was; a pick and Done applies it (saved, and the screens come back in it); z_fonts_kept starts again
# from what this run saved. Shots of the sheet with each family highlighted, upright and sideways.
# A tap on a family refreshes the sample and the two rows whose highlight changed, never the whole
# screen (font_step: the sample's top through the lower row, whatever the font); Done on another font
# refreshes the whole screen with a Full, as a style change does. ("has", not "is": a refresh of the
# Library starting up can land in the same wait.)
# rust-only
wait 100
expect settings.font is terminess
expect font is terminess
tap 1535 95
wait 50
expect library.sheet is settings
expect library.ui_has.font is 1
shot 01-settings-terminess
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
shot 02-font-terminess
expect display.presents not -1
tap library font.atkinson
wait 50
expect library.font.pick is atkinson
expect font is terminess
expect display.presents has ui 106 566 1648 600
shot 03-font-atkinson
expect display.presents not -1
tap library font.noto-sans
wait 50
expect library.font.pick is noto-sans
expect display.presents has ui 106 566 1648 746
shot 04-font-noto-sans
expect display.presents not -1
tap library font.literata
wait 50
expect library.font.pick is literata
expect display.presents has ui 106 566 1648 892
shot 05-font-literata
rotate 1
wait 100
expect library.sheet is font
shot 06-font-literata-sideways
rotate 0
wait 100
# Cancel: nothing changes
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is terminess
expect settings.font is terminess
# The sheet opens on the font in use again; Esc is Cancel too
tap library font
wait 50
expect library.font.pick is terminess
tap library font.literata
wait 50
key 1 0
wait 50
expect library.sheet is settings
expect font is terminess
# A pick and Done: applied and saved, the chip names it
tap library font
wait 50
tap library font.atkinson
wait 50
expect display.presents not -1
tap library font.done
wait 100
expect library.sheet is settings
expect display.presents has full 0 0 1860 2480
expect font is atkinson
expect settings.font is atkinson
shot 07-settings-atkinson
rotate 1
wait 100
shot 08-settings-atkinson-sideways
rotate 0
wait 100
# Done on the font in use changes nothing
tap library font
wait 50
expect library.font.pick is atkinson
tap library font.done
wait 50
expect font is atkinson
# Gothic has no chip, and keeps its own faces; the font stays picked for Classic
tap 930 930
wait 100
expect settings.theme is gothic
expect library.ui_has.font is 0
expect settings.font is atkinson
shot 09-settings-gothic
tap 930 930
wait 100
expect settings.theme is classic
expect library.ui_has.font is 1
# Literata for z_fonts_kept
tap library font
wait 50
tap library font.literata
wait 50
tap library font.done
wait 100
expect settings.font is literata
tap library done
wait 50
expect library.sheet is none
shot 10-library-literata
quit
