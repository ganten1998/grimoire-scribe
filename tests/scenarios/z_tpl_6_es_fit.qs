# settings: status_mode=1
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# The Templates sheet with templates in it, in Spanish with the fit check (a run that cuts any
# text short fails), in Gothic and then Classic, upright and sideways: over New notebook and over
# the editor's page, with its pages of tiles and their buttons. (make l10n-check runs the same taps,
# tests/l10n/templates.qs, in every language and style; this keeps a Spanish run in make scenarios,
# with the style changed while Grimoire runs.) Its taps land in both styles.
# copy: tests/fixtures/tpl-many templates
# rust-only
wait 300
tap 965 95
wait 100
expect library.sheet is new
shot 01-new-gothic
tap 300 1650
wait 200
expect library.sheet is templates
# (50 previews made in the background: waitbg waits for them, however long the machine takes)
waitbg
wait 300
expect templates.ready is 50
shot 02-templates-gothic
tap 1640 2123
wait 200
expect templates.page is 2
shot 03-templates-gothic-page-2
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
shot 04-templates-gothic-sideways
tap 2260 1694
wait 200
expect templates.page is 2
shot 05-templates-gothic-sideways-page-2
tap 300 1694
wait 100
tap 570 1700
wait 100
expect library.sheet is none
rotate 0
wait 300
# Classic (Settings, its style row, Done), the same again.
tap 1535 95
wait 100
expect library.sheet is settings
tap 930 930
wait 100
expect settings.theme is classic
tap 1560 1930
wait 100
expect library.sheet is none
tap 965 95
wait 100
expect library.sheet is new
shot 06-new-classic
tap 300 1650
wait 200
expect library.sheet is templates
expect templates.ready is 50
shot 07-templates-classic
tap 1640 2123
wait 200
expect templates.page is 2
tap 300 2123
wait 100
tap 300 2250
wait 100
expect library.sheet is none
rotate 1
wait 300
tap 1585 95
wait 100
tap 1462 1116
wait 200
expect library.sheet is templates
shot 08-templates-classic-sideways
tap 2260 1694
wait 200
expect templates.page is 2
tap 300 1694
wait 100
tap 570 1700
wait 100
rotate 0
wait 300
# Over the editor's page, from its Paper sheet, upright and sideways.
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
pentap panel paper
wait 200
expect editor.modal is paper
shot 09-paper-classic
pentap 232 1750
wait 200
expect editor.modal is templates
shot 10-templates-editor-classic
pentap 300 2123
wait 200
expect editor.modal is paper
tap 1562 2125
wait 200
rotate 1
wait 400
pentap panel paper
wait 200
expect editor.modal is paper
shot 11-paper-classic-sideways
tap 1544 1300
wait 200
expect editor.modal is templates
tap 2260 1694
wait 200
expect templates.page is 2
shot 12-templates-editor-classic-sideways
tap 300 1694
wait 200
tap 2182 1696
wait 200
expect editor.modal is none
rotate 0
wait 300
quit
