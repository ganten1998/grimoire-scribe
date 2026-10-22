# 50 templates and a file that is no picture: the Templates sheet opens at once, its tiles waiting
# for the previews the background makes, which fill in as they come (the file that is no picture
# leaves the sheet). Three pages of 24 upright, 21 sideways, with Prev and Next. A turn and a sleep
# keep the sheet open with the same tiles in view: after a turn it shows the page with the first
# tile it showed. The same over New notebook.
# copy: tests/fixtures/tpl-many templates
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
pentap 1768 1242
wait 200
pentap 232 1750
wait 200
expect editor.modal is templates
expect templates.page is 1
expect templates.ready is 0
shot 01-waiting
# (50 previews made in the background: waitbg waits for them, however long the machine takes)
waitbg
wait 300
expect templates.count is 50
expect templates.ready is 50
expect templates.pages is 3
shot 02-filled-in
pentap 1640 2123
wait 200
expect templates.page is 2
shot 03-page-2
rotate 1
wait 400
expect editor.modal is templates
expect templates.pages is 3
expect templates.page is 2
shot 04-sideways-page-2
sleep
wait 200
wake
wait 500
expect editor.modal is templates
expect templates.page is 2
shot 05-sideways-after-a-wake
pentap 2260 1694
wait 200
expect templates.page is 3
shot 06-sideways-page-3
pentap 1600 1694
wait 200
expect templates.page is 2
pentap 2260 1694
wait 200
expect templates.page is 3
# Upright again: page 3 sideways starts with Grid 43, on page 2 upright.
rotate 0
wait 400
expect editor.modal is templates
expect templates.page is 2
shot 07-upright-page-2
# Grid 43 (page 2's 19th tile) becomes the page's paper.
pentap 232 1748
wait 300
expect editor.modal is none
expect editor.paper_hex has 0x50
waitbg
wait 300
expect editor.picture is shown
shot 08-on-grid-43
pentap 1768 1242
wait 200
pentap 232 1750
wait 300
expect templates.selected is Grid 43
expect templates.page is 2
pentap 300 2123
wait 200
tap 1562 2125
wait 200
expect editor.modal is none
# Over New notebook: a turn and a wake keep it open there too.
pentap 1768 1566
wait 600
expect screen is library
tap 965 95
wait 100
tap 300 1650
wait 200
expect library.sheet is templates
expect templates.page is 1
tap 1640 2123
wait 200
expect templates.page is 2
rotate 1
wait 400
expect library.sheet is templates
expect templates.page is 2
shot 09-new-notebook-sideways
sleep
wait 200
wake
wait 500
expect library.sheet is templates
expect templates.page is 2
tap 300 1694
wait 200
expect library.sheet is new
tap 570 1700
wait 100
expect library.sheet is none
rotate 0
wait 300
quit
