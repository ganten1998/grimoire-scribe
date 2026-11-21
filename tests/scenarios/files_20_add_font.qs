# settings: status_mode=1
# settings: theme=classic
# settings: files_card=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_BG_DELAY_MS=1200
# env: GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/fonts/good fonts
# copy: tests/fixtures/fonts/good documents/Typefaces
# copy: tests/fixtures/fonts/refused documents/Odd
# Add a font... in Settings > Font (2.2, issue #161). The row comes after the last of your fonts (under the text
# while the folder is empty) and opens the font pick over the sheet: the Kindle's own fonts folder first, then
# Documents and All storage, fonts only (.ttf, .otf: no web font), each by its name, size and date. A font's card
# there is Add to fonts and Cancel. The copy asked, the sheet is back: the row waits (an hourglass, no tap) until the
# copy job has ended, then the folder is read again and the new file is a row that says "Checking..." under the
# notice "Checking 1 font...", takes no tap and is no font yet (Grimoire stays in Terminess and holds no font file).
# When its check has ended the row shows the font and can be tapped; it is listed, not applied. A tap applies it, as
# any row's does. A name taken in grimoire/fonts is not replaced (the copy is " 2"); a file its check refuses is a
# row that says why and takes no tap. Back at the pick's root comes back to the sheet with nothing changed. The
# Kindle's folder is as it was. GRIMOIRE_BG_DELAY_MS holds each job back so the waits can be seen. Over a notebook's
# page (Settings opened from the editor) the sheet has no such row.
# rust-only
wait 100
expect font is terminess
tap library settings
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.rows is 0
expect library.font.page is 1/1
expect library.ui_has.font.add is 1
expect library.font.copying is 0
shot 01-sheet-empty
# the pick: its root, and Back out of it
tap library font.add
wait 300
expect files.open is 1
expect files.mode is pick
expect files.place is root
expect files.rows is 3
expect files.row.1 is Kindle fonts
expect files.sub.1 is fonts
expect files.row.2 is Documents
expect files.row.3 is All storage
expect notice is
shot 02-pick-root
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is font
expect library.font.rows is 0
expect library.font.copying is 0
expect bg.busy is 0
# the Kindle's own fonts: names, sizes and dates; a font's card
tap library font.add
wait 300
tap library files.item.Kindle_fonts
wait 300
expect files.place is fonts
expect files.crumbs is Files > Kindle fonts
expect files.order is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf
expect files.kind.2 is font
expect files.sub.2 has 17 KB,
shot 03-kindle-fonts
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
expect files.card.details has 17 KB,
expect files.card.rows is Add to fonts|Cancel
shot 04-card
# Add to fonts: back at the sheet, the row waiting for the copy
tap library files.card.add
wait 300
expect files.open is 0
expect library.sheet is font
expect library.font.copying is 1
expect library.ui_has.font.add is 0
expect library.font.rows is 0
shot 05-copying
# the copy is in: listed, being checked, no font yet
waitfor library.font.copying is 0
expect toast is
expect library.font.rows is 1
expect library.font.state.GrimoireTest-Regular.ttf is checking
expect library.font.note.GrimoireTest-Regular.ttf is Checking...
expect library.ui_has.font.file:GrimoireTest-Regular.ttf is 0
expect library.ui_has.font.add is 1
expect library.font.sample is family
expect font is terminess
expect font.own.bytes is 0
waitfor notice is Checking 1 font...
expect notice.key is fontcheck
shot 06-checking
# its check has ended: a row that can be tapped; listed, not applied
waitfor library.font.checking is 0
waitfor notice is
expect library.font.state.GrimoireTest-Regular.ttf is ready
expect library.ui_has.font.file:GrimoireTest-Regular.ttf is 1
expect library.font.pick is terminess
expect font is terminess
expect settings.font is terminess
expect font.own.bytes is 0
expect storage.names.grimoire/fonts is .cache|GrimoireTest-Regular.ttf|README.txt
expect storage.size.grimoire/fonts/GrimoireTest-Regular.ttf is 16844
shot 07-listed
# from another folder: Documents
tap library font.add
wait 300
tap library files.item.documents
wait 300
tap library files.item.Typefaces
wait 300
expect files.place is documents/Typefaces
tap library files.item.GrimoireTestCFF.otf
wait 300
expect files.card.rows is Add to fonts|Cancel
tap library files.card.add
wait 300
expect library.sheet is font
expect library.font.copying is 1
waitfor library.font.copying is 0
expect library.font.rows is 2
expect library.font.state.GrimoireTestCFF.otf is checking
expect library.ui_has.font.file:GrimoireTestCFF.otf is 0
waitfor library.font.checking is 0
expect library.font.state.GrimoireTestCFF.otf is ready
expect library.font.shown is GrimoireTest-Regular.ttf|GrimoireTestCFF.otf
expect font is terminess
shot 08-two-fonts
# a tap applies a font that passed, as any row's does
tap library font.file:GrimoireTest-Regular.ttf
wait 100
expect font is file:GrimoireTest-Regular.ttf
expect settings.font is file:GrimoireTest-Regular.ttf
expect library.font.sample is own
expect library.ui_has.font.add is 1
shot 09-applied
rotate 1
wait 100
expect library.sheet is font
expect library.ui_has.font.add is 1
shot 10-sideways
rotate 0
wait 100
# a name taken in grimoire/fonts: the copy is " 2", and the file that had the name is as it was
tap library font.add
wait 300
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.add
wait 300
expect library.font.copying is 1
waitfor library.font.copying is 0
expect library.font.rows is 3
expect library.font.shown is GrimoireTest-Regular 2.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf
expect storage.size.grimoire/fonts/GrimoireTest-Regular.ttf is 16844
waitfor library.font.checking is 0
expect font is file:GrimoireTest-Regular.ttf
# a file its check refuses: copied as any file is, then a row that says why and takes no tap; a web font is no
# row of the pick
tap library font.add
wait 300
tap library files.item.documents
wait 300
tap library files.item.Odd
wait 300
expect files.order is Blank.ttf|Borrowing.ttf|Cut.ttf|Symbols.ttf|WebInside.ttf
tap library files.item.Cut.ttf
wait 300
expect files.card.rows is Add to fonts|Cancel
tap library files.card.add
wait 300
expect library.font.copying is 1
waitfor library.font.copying is 0
expect library.font.state.Cut.ttf is checking
waitfor library.font.checking is 0
expect library.font.state.Cut.ttf is unreadable
expect library.font.note.Cut.ttf is Unreadable font.
expect library.ui_has.font.file:Cut.ttf is 0
expect library.font.rows is 4
expect font is file:GrimoireTest-Regular.ttf
expect library.ui_has.font.add is 1
shot 11-refused
# the Kindle's folder and the folder in Documents: as they were
expect storage.names.fonts is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf
expect storage.size.fonts/GrimoireTest-Regular.ttf is 16844
expect storage.names.documents/Typefaces is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf
tap library font.done
wait 100
expect library.sheet is none
# over a notebook's page the sheet has no Add a font... row
tap library add
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
tap panel settings
wait 300
expect editor.modal is settings
tap library settings.cat.1
wait 100
tap library font
wait 100
expect library.sheet is font
expect library.font.rows is 4
expect library.ui_has.font.add is 0
expect library.font.page is 1/1
shot 12-over-a-notebook
tap library font.done
wait 300
expect editor.modal is none
quit
