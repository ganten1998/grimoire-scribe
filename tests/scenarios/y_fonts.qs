# settings: status_mode=1
# settings: theme=classic
# settings: shortcuts_enabled=1
# env: GRIMOIRE_CHECK_FIT=1
# Settings > Font (2.1, Classic only; the Font row of Settings' Display category since the new Settings
# menu, #115): the row opens the Font sheet with the font in use highlighted; a tap on a family applies it at
# once and saves it (settings.font follows), and the screen comes back in it with a Full refresh, as a style
# change does; the sheet's Back (and Esc) goes back to Settings keeping the font that was tapped; its X
# closes all of Settings, the font kept. z_fonts_kept starts again from what this run saved: the last font
# is tapped and the run then goes Back and quits with the storage full, so only the tap can have saved it.
# Shots of the sheet in each family, upright and sideways. ("has", not "is": a refresh of the Library
# starting up can land in the same wait.)
# rust-only
wait 100
# (the clock kept still: no minute's redraw of the clock corner lands in a refresh count)
clock 1700000000
expect settings.font is terminess
expect font is terminess
tap 1535 95
wait 50
expect library.sheet is settings
tap library settings.cat.1
wait 50
expect library.ui_has.font is 1
shot 01-settings-terminess
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
# (no font of the owner's own: the sheet says where to copy one)
expect library.font.rows is 0
expect library.font.sample is family
shot 02-font-terminess
# A tap applies the family at once: saved, and the screen drawn again in it with a Full refresh
expect display.presents not -1
tap library font.atkinson
wait 50
expect library.sheet is font
expect library.font.pick is atkinson
expect font is atkinson
expect settings.font is atkinson
expect display.presents has full 0 0 1860 2480
shot 03-font-atkinson
expect display.presents not -1
tap library font.noto-sans
wait 50
expect library.font.pick is noto-sans
expect font is noto-sans
expect settings.font is noto-sans
expect display.presents has full 0 0 1860 2480
shot 04-font-noto-sans
expect display.presents not -1
tap library font.literata
wait 50
expect library.font.pick is literata
expect font is literata
expect settings.font is literata
expect display.presents has full 0 0 1860 2480
shot 05-font-literata
rotate 1
wait 100
expect library.sheet is font
expect library.font.pick is literata
shot 06-font-literata-sideways
rotate 0
wait 100
# Back: to Settings, the font tapped kept
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is literata
expect settings.font is literata
# The sheet opens on the font in use again; Esc is Back too, and keeps the font tapped
tap library font
wait 50
expect library.font.pick is literata
tap library font.terminess
wait 50
expect font is terminess
expect settings.font is terminess
key 1 0
wait 50
expect library.sheet is settings
expect font is terminess
expect settings.font is terminess
# A tap and the X: applied and saved at the tap; the X closes all of Settings with the font kept: its
# pressed look, then one ui refresh of the Library, no second Full (the font changed already)
tap library font
wait 50
tap library font.atkinson
wait 50
expect font is atkinson
expect display.presents has full 0 0 1860 2480
tap library font.done
wait 100
expect library.sheet is none
expect display.presents is fast 1660 20 150 150, ui 0 0 1860 2480
expect font is atkinson
expect settings.font is atkinson
# Settings again, in Atkinson, its Font row naming it
tap 1535 95
wait 50
tap library settings.cat.1
wait 50
shot 07-settings-atkinson
rotate 1
wait 100
shot 08-settings-atkinson-sideways
rotate 0
wait 100
# The X on the font in use changes nothing
tap library font
wait 50
expect library.font.pick is atkinson
tap library font.done
wait 50
expect library.sheet is none
expect font is atkinson
expect settings.font is atkinson
# Gothic has no Font row, and keeps its own faces; the font stays picked for Classic
tap 1535 95
wait 50
tap library settings.cat.1
wait 50
tap library theme
wait 100
expect settings.theme is gothic
expect library.ui_has.font is 0
expect settings.font is atkinson
shot 09-settings-gothic
tap library theme
wait 100
expect settings.theme is classic
expect library.ui_has.font is 1
# Literata for z_fonts_kept: tapped, then Back; from here on no file can be written, so the font z_fonts_kept
# starts in was saved by the tap
tap library font
wait 50
tap library font.literata
wait 50
expect settings.font is literata
storage full
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is literata
tap library done
wait 50
expect library.sheet is none
shot 10-library-literata
quit
