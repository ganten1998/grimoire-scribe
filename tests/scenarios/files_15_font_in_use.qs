# settings: status_mode=1
# settings: files_card=1
# settings: theme=classic
# settings: font=file:GrimoireTest-Regular.ttf
# settings: font_bold=file:GrimoireTest-Bold.ttf
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/fonts/good grimoire/fonts
# The font in use, in Files (2.2, issue #144). Settings names Classic's own font and its Bold file by their paths
# under grimoire/fonts, so: Delete of either says "Font in use.", asks nothing and moves nothing; another font goes
# to the trash as any file. Rename takes the setting along in the same step: settings.ini names the new path, the
# font still draws, and nothing is checked again (what its check found went with the file: its card names its
# family at once, and the Font sheet has it ready and picked). files_16_font_kept starts again from what this run
# saved; the start that needs no new check is test_sec_font_verdict.rs's (the same data root, no worker to be had). The keys are files_8_rename's (the digit 2 at 310 1725, Done at 1685 2365).
# rust-only
wait 100
waitbg
waitfor font is file:GrimoireTest-Regular.ttf
expect settings.font_bold is GrimoireTest-Bold.ttf
tap library files-card
wait 300
tap library files.item.Fonts
wait 300
expect files.place is grimoire/fonts
expect files.order is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf
# Delete of the font in use, and of its Bold file: said, nothing asked, nothing moved
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card is font
expect files.card.details has Font, Grimoire Test,
expect files.card.rows is Rename|Delete|Cancel
tap library files.card.trash
wait 300
expect toast is Font in use.
expect files.card is
shot 01-in-use
tap library files.item.GrimoireTest-Bold.ttf
wait 300
tap library files.card.trash
wait 300
expect toast is Font in use.
expect files.card is
expect storage.names.grimoire/fonts is .cache|GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf|README.txt
expect storage.names.grimoire/trash is
# a font Settings does not name goes as any file
tap library files.item.GrimoireTestCFF.otf
wait 300
tap library files.card.trash
wait 500
expect files.card is delete
tap library files.card.delete
wait 300
expect toast is Moved to grimoire/trash
expect files.order is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestVariable.ttf
# Rename of the font in use: Settings has the new path, the font still draws, nothing is checked again
tap library files.item.GrimoireTest-Regular.ttf
wait 300
tap library files.card.rename
wait 300
expect files.text is GrimoireTest-Regular.ttf
tap 310 1725
expect files.text is GrimoireTest-Regular2.ttf
tap 1685 2365
wait 300
expect files.keyboard is 0
expect storage.exists.grimoire/fonts/GrimoireTest-Regular2.ttf is 1
expect storage.exists.grimoire/fonts/GrimoireTest-Regular.ttf is 0
expect settings.font is file:GrimoireTest-Regular2.ttf
expect font is file:GrimoireTest-Regular2.ttf
expect bg.busy is 0
expect notice.key not fontcheck
expect files.order is GrimoireTest-Bold.ttf|GrimoireTest-Regular2.ttf|GrimoireTestVariable.ttf
shot 02-renamed
tap library files.item.GrimoireTest-Regular2.ttf
wait 300
expect files.card.details has Font, Grimoire Test,
# under its new name it is still the font in use
tap library files.card.trash
wait 300
expect toast is Font in use.
# its Bold file renamed: that setting follows too
tap library files.item.GrimoireTest-Bold.ttf
wait 300
tap library files.card.rename
wait 300
tap 310 1725
expect files.text is GrimoireTest-Bold2.ttf
tap 1685 2365
wait 300
expect settings.font_bold is GrimoireTest-Bold2.ttf
expect settings.font is file:GrimoireTest-Regular2.ttf
expect font is file:GrimoireTest-Regular2.ttf
expect bg.busy is 0
# the Font sheet: the renamed file is the pick, ready, and nothing is being checked
tap library files.back
wait 300
tap library files.back
wait 300
expect files.open is 0
tap library settings
wait 50
expect library.sheet is settings
tap library settings.cat.1
wait 50
tap library font
wait 50
expect library.sheet is font
expect library.font.pick is file:GrimoireTest-Regular2.ttf
expect library.font.state.GrimoireTest-Regular2.ttf is ready
expect library.font.state.GrimoireTest-Bold2.ttf is ready
expect library.font.checking is 0
shot 03-font-sheet
tap library font.cancel
wait 50
tap library done
wait 50
expect font is file:GrimoireTest-Regular2.ttf
quit
