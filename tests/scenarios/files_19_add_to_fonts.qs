# settings: status_mode=1
# settings: files_card=1
# settings: theme=classic
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/fonts/good fonts
# copy: tests/fixtures/fonts/good documents/Type
# copy: tests/fixtures/fonts/refused documents/Odd
# Add to fonts on a font's card in Files (2.2, issue #161). In the Kindle's own fonts folder a font's card is Add to
# fonts and Cancel, with no Rename and no Delete: the folder is read only, and stays as it was. The row copies the
# file into grimoire/fonts by the copy job ("Copied to Fonts." when it is in), and the font check then runs on it by
# itself, as on a file copied over USB: nothing is applied, the font is one more row of Settings > Font, which a tap
# applies. A name taken there is never replaced (" 2"). A file that is no font Grimoire can read is copied as any
# file and refused by its check: its row says why and takes no tap. A font of grimoire/fonts has no Add to fonts
# (it is one already), and a font in the trash has Restore alone.
# rust-only
wait 300
expect font is terminess
tap library files-card
wait 300
tap library files.item.Kindle_fonts
wait 300
expect files.place is fonts
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
expect files.card.details has Font, 17 KB,
expect files.card.rows is Add to fonts|Cancel
shot 01-kindle-font-card
tap library files.card.add
waitbg
wait 200
expect files.card is
expect files.place is fonts
expect toast is Copied to Fonts.
expect storage.exists.grimoire/fonts/GrimoireTest-Regular.ttf is 1
expect storage.size.grimoire/fonts/GrimoireTest-Regular.ttf is 16844
shot 02-copied
# the check runs by itself (its results are in the folder's .cache): nothing is applied
waitbg
wait 200
waitbg
expect storage.names.grimoire/fonts is .cache|GrimoireTest-Regular.ttf|README.txt
expect font is terminess
expect settings.font is terminess
expect font.own.bytes is 0
# again: the name is taken, the copy is " 2", and the first file is as it was
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.add
waitbg
wait 200
expect toast is Copied to Fonts.
expect storage.names.grimoire/fonts is .cache|GrimoireTest-Regular 2.ttf|GrimoireTest-Regular.ttf|README.txt
expect storage.size.grimoire/fonts/GrimoireTest-Regular.ttf is 16844
# the Kindle's folder: as it was
expect storage.names.fonts is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf
expect storage.size.fonts/GrimoireTest-Regular.ttf is 16844
# from another folder, and a file its check will refuse
tap library files.crumb.1
wait 300
tap library files.item.documents
wait 300
tap library files.item.Type
wait 300
tap library files.item.GrimoireTestCFF.otf
wait 300
expect files.card.rows is Add to fonts|Cancel
tap library files.card.add
waitbg
wait 200
expect toast is Copied to Fonts.
tap library files.back
wait 300
tap library files.item.Odd
wait 300
expect files.order is Blank.ttf|Borrowing.ttf|Cut.ttf|Symbols.ttf|WebInside.ttf
tap library files.item.Cut.ttf
wait 300
expect files.card.rows is Add to fonts|Cancel
tap library files.card.add
waitbg
wait 200
expect toast is Copied to Fonts.
waitbg
wait 200
waitbg
# in grimoire/fonts: no Add to fonts, and the card of a font that passed names its family
tap library files.crumb.1
wait 300
tap library files.item.Fonts
wait 300
expect files.order is Cut.ttf|GrimoireTest-Regular 2.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf
tap library files.item.GrimoireTestCFF.otf
wait 300
expect files.card.details has Font, Grimoire Test CFF, 18 KB,
expect files.card.rows is Rename|Delete|Cancel
shot 03-in-grimoire-fonts
tap library files.card.cancel
wait 200
tap library files.item.Cut.ttf
wait 300
expect files.card.details has Font, 1 KB,
expect files.card.rows is Rename|Delete|Cancel
# into the trash, where a font has Restore alone
tap library files.card.trash
wait 500
expect files.card is delete
tap library files.card.delete
wait 300
expect toast is Moved to grimoire/trash
tap library files.crumb.1
wait 300
tap library files.item.Trash
wait 400
expect files.rows is 1
tap library files.row.1
wait 300
expect files.card is trashed
expect files.card.rows is Restore|Cancel
shot 04-in-the-trash
tap library files.card.restore
wait 300
expect toast is Restored to grimoire/fonts.
# Settings > Font lists them: the three that passed can be tapped, the one refused says why; nothing was applied
tap library files.back
wait 300
tap library files.back
wait 300
expect files.open is 0
tap 1535 95
wait 50
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is terminess
expect library.font.rows is 4
waitbg
waitfor library.font.checking is 0
expect library.font.shown is Cut.ttf|GrimoireTest-Regular 2.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf
expect library.font.state.GrimoireTest-Regular.ttf is ready
expect library.font.state.GrimoireTestCFF.otf is ready
expect library.font.state.Cut.ttf is unreadable
expect library.font.note.Cut.ttf is Unreadable font.
expect library.ui_has.font.file:Cut.ttf is 0
expect library.ui_has.font.file:GrimoireTestCFF.otf is 1
expect font is terminess
shot 05-font-sheet
tap library font.file:GrimoireTestCFF.otf
wait 100
expect font is file:GrimoireTestCFF.otf
expect settings.font is file:GrimoireTestCFF.otf
tap library font.cancel
wait 50
tap library done
wait 50
quit
