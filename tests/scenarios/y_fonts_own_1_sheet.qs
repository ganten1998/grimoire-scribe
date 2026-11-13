# settings: status_mode=1
# settings: theme=classic
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/fonts/good fonts
# copy: tests/fixtures/fonts/refused fonts
# Your own fonts (2.1.0-beta.2): font files copied into grimoire/fonts show in Settings > Font under "Your
# fonts", after the fonts that come with Grimoire ("Built in"). While the background check runs, the rows say
# "Checking..." and a notice stays above the buttons ("Checking 9 fonts..."); then each font shows its name
# drawn in itself (the picture the check made), or its row says why it cannot be used: "Unreadable font." (cut
# short, or letters that draw nothing), "No Latin letters." (a symbol font), "Use .ttf or .otf." (a web font, by
# its name or by its bytes). Rows that cannot be used are greyed and take no tap. A tap on a font highlights it
# and draws the sample in it, and changes nothing else; Cancel leaves the font as it was; Done applies it: saved
# as font=file:<its path>, the screens come back in it with one Full refresh, the chip names its family, and
# the crash guard's mark is gone once that screen is on the panel. The sheet's own words are Terminess while a
# font of the owner's own is in use. A built-in font picked again takes the custom one away.
# z_fonts_own_kept starts again from what this run saved.
# rust-only
wait 100
expect font is terminess
expect font.own.bytes is 0
tap 1535 95
wait 50
expect library.sheet is settings
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
expect library.font.rows is 10
expect library.font.checking is 9
expect library.font.page is 1/2
expect library.font.shown is Blank.ttf|Borrowing.ttf|Cut.ttf|GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf
expect library.font.state.Blank.ttf is checking
expect library.font.note.Blank.ttf is Checking...
expect library.font.state.Web.woff2 is format
expect library.ui_has.font.file:GrimoireTest-Regular.ttf is 0
waitfor notice is Checking 9 fonts...
expect notice.key is fontcheck
shot 01-checking
# The check ends: the rows fill in, and the notice goes
waitbg
waitfor library.font.checking is 0
waitfor notice is
expect library.font.state.GrimoireTest-Regular.ttf is ready
expect library.font.state.GrimoireTest-Bold.ttf is ready
expect library.font.state.GrimoireTestCFF.otf is ready
expect library.font.state.GrimoireTestVariable.ttf is ready
expect library.font.state.Blank.ttf is unreadable
expect library.font.note.Blank.ttf is Unreadable font.
expect library.font.state.Cut.ttf is unreadable
expect library.font.state.Symbols.ttf is nolatin
expect library.font.note.Symbols.ttf is No Latin letters.
expect library.font.note.Web.woff2 is Use .ttf or .otf.
expect library.font.state.WebInside.ttf is format
expect library.font.state.Borrowing.ttf is ready
expect library.font.note.Borrowing.ttf is Some letters come from Terminess.
expect library.ui_has.font.file:GrimoireTest-Regular.ttf is 1
expect library.ui_has.font.file:Blank.ttf is 0
expect library.ui_has.font.file:Cut.ttf is 0
shot 02-checked
# The second page of them
tap library font.next
wait 50
expect library.font.page is 2/2
expect library.font.shown is GrimoireTestVariable.ttf|Symbols.ttf|Web.woff2|WebInside.ttf
expect library.ui_has.font.file:Symbols.ttf is 0
expect library.ui_has.font.file:Web.woff2 is 0
expect library.ui_has.font.file:WebInside.ttf is 0
shot 03-second-page
tap library font.prev
wait 50
expect library.font.page is 1/2
# A tap highlights a font and draws the sample in it; nothing is applied
expect display.presents not -1
tap library font.file:GrimoireTest-Regular.ttf
wait 50
expect library.font.pick is file:GrimoireTest-Regular.ttf
expect library.font.sample is own
expect font is terminess
expect settings.font is terminess
expect display.presents not full
shot 04-picked
rotate 1
wait 100
expect library.sheet is font
expect library.font.pick is file:GrimoireTest-Regular.ttf
shot 05-picked-sideways
rotate 0
wait 100
# Cancel: nothing changes, and the sample font is let go
tap library font.cancel
wait 50
expect library.sheet is settings
expect font is terminess
expect font.own.bytes is 0
# Done: applied and saved
tap library font
wait 50
expect library.font.pick is terminess
expect library.font.checking is 0
tap library font.file:GrimoireTest-Regular.ttf
wait 50
expect display.presents not -1
tap library font.done
wait 100
expect library.sheet is settings
expect display.presents has full 0 0 1860 2480
expect font is file:GrimoireTest-Regular.ttf
expect settings.font is file:GrimoireTest-Regular.ttf
expect font.own.drawn is 1
expect font.guard is
shot 06-settings-own
# The sheet again: its own words in Terminess, the font in use highlighted and checked
tap library font
wait 50
expect library.font.pick is file:GrimoireTest-Regular.ttf
expect library.font.sample is own
shot 07-sheet-own-in-use
# The variable font: its bold comes from its weight axis
tap library font.next
wait 50
tap library font.file:GrimoireTestVariable.ttf
wait 50
expect library.font.sample is own
tap library font.done
wait 100
expect font is file:GrimoireTestVariable.ttf
expect settings.font is file:GrimoireTestVariable.ttf
shot 08-settings-variable
tap library done
wait 50
shot 09-library-variable
# Gothic holds no custom font, and Classic has it back
tap 1535 95
wait 50
tap 600 930
wait 100
expect settings.theme is gothic
expect font.own.bytes is 0
expect settings.font is file:GrimoireTestVariable.ttf
tap 600 930
wait 100
expect settings.theme is classic
expect font is file:GrimoireTestVariable.ttf
# A built-in font takes the custom one away
tap library font
wait 50
tap library font.noto-sans
wait 50
expect library.font.sample is family
tap library font.done
wait 100
expect font is noto-sans
expect settings.font is noto-sans
expect font.own.bytes is 0
# The OpenType (CFF) one, with the Bold file of the test family beside it, for z_fonts_own_kept
tap library font
wait 50
tap library font.file:GrimoireTestCFF.otf
wait 50
tap library font.done
wait 100
expect font is file:GrimoireTestCFF.otf
tap library font
wait 50
tap library font.file:GrimoireTest-Regular.ttf
wait 50
tap library font.done
wait 100
expect settings.font is file:GrimoireTest-Regular.ttf
tap library done
wait 50
expect library.sheet is none
quit
