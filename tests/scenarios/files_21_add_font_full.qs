# settings: status_mode=1
# settings: theme=classic
# settings: files_card=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_BG_DELAY_MS=800
# env: GRIMOIRE_COPY_FAIL_AFTER=4096
# copy: tests/fixtures/fonts/good fonts
# Settings > Font's Add a font... (2.2, issue #161) when the copy job's writes fail part way, as on a full storage:
# the row waits while the job runs, then "Couldn't copy file. Is the storage full?", the sheet as it was (the row
# back and taking taps, no font listed, none checked, none applied), and nothing left in grimoire/fonts (the copy's
# temporary file, which held the font's first bytes, is gone). The Kindle's folder is as it was.
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
tap library font.add
wait 300
tap library files.item.Kindle_fonts
wait 300
tap library files.item.GrimoireTest-Regular.ttf
wait 300
expect files.card.rows is Add to fonts|Cancel
tap library files.card.add
wait 300
expect files.open is 0
expect library.sheet is font
expect library.font.copying is 1
expect library.ui_has.font.add is 0
waitfor library.font.copying is 0
expect toast is Couldn't copy file. Is the storage full?
expect library.font.rows is 0
expect library.font.checking is 0
expect library.ui_has.font.add is 1
expect notice is
expect font is terminess
expect settings.font is terminess
expect storage.names.grimoire/fonts is README.txt
expect storage.names.fonts is GrimoireTest-Bold.ttf|GrimoireTest-Regular.ttf|GrimoireTestCFF.otf|GrimoireTestVariable.ttf
shot 01-copy-failed
tap library font.done
wait 100
expect library.sheet is none
quit
