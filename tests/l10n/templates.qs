# make l10n-check runs this in every language, Gothic and Classic, with GRIMOIRE_CHECK_FIT=1: the
# Templates sheet with templates in it (50, and a file that is no picture, copied in first by the
# "# copy:" line below, as make scenarios does), over New notebook and over the editor's page,
# upright and sideways, with its pages of tiles and their buttons. (y_all_screens visits the
# sheet's other state, with no templates.) The taps are z_tpl_6_es_fit's; they land in both styles.
# (The Rust build runs templates-rust.qs instead: its panel has one more button, and the Paper button moved.)
# copy: tests/fixtures/tpl-many templates
wait 300
tap 965 95
wait 100
expect library.sheet is new
tap 300 1650
wait 200
expect library.sheet is templates
# (50 previews made in the background: waitbg waits for them, however long the machine takes)
waitbg
wait 300
expect templates.ready is 50
shot 01-templates
tap 1640 2123
wait 200
expect templates.page is 2
shot 02-templates-page-2
tap 300 2123
wait 100
expect library.sheet is new
tap 300 2250
wait 100
expect library.sheet is none
rotate 1
wait 300
tap 1585 95
wait 100
expect library.sheet is new
tap 1462 1116
wait 200
expect library.sheet is templates
shot 03-templates-sideways
tap 2260 1694
wait 200
expect templates.page is 2
shot 04-templates-sideways-page-2
tap 300 1694
wait 100
tap 570 1700
wait 100
expect library.sheet is none
rotate 0
wait 300
# Over the editor's page, from its Paper sheet, upright and sideways.
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
pentap 1768 1242
wait 200
expect editor.modal is paper
pentap 232 1750
wait 200
expect editor.modal is templates
shot 05-templates-editor
pentap 300 2123
wait 200
expect editor.modal is paper
tap 1562 2125
wait 200
rotate 1
wait 400
pentap 2388 1244
wait 200
expect editor.modal is paper
tap 1544 1300
wait 200
expect editor.modal is templates
tap 2260 1694
wait 200
expect templates.page is 2
shot 06-templates-editor-sideways-page-2
tap 300 1694
wait 200
tap 2182 1696
wait 200
expect editor.modal is none
rotate 0
wait 300
quit
