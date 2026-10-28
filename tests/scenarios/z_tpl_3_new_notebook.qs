# New notebook > More papers... > a template: New notebook's last tile shows it, selected, and the
# notebook is made on it. A paper tile picked after it takes its place, and the template again. The
# first page draws white until the background has made the template's picture for the notebook,
# then draws it; a page added after it gets it too, and the Library's card shows it. (The jobs are
# slowed here, as a big picture is on the Kindle, so that the page is still white right after
# Create; the waits hold for them on a busy machine.)
# copy: tests/fixtures/scen-templates templates
# env: GRIMOIRE_BG_DELAY_MS=3000
# rust-only
wait 300
tap 965 95
wait 100
expect library.sheet is new
tap 300 1650
wait 200
expect library.sheet is templates
expect templates.count is 2
waitbg
wait 300
expect templates.ready is 2
tap 511 452
wait 200
expect library.sheet is new
expect library.paper is template
expect library.template is Lined
shot 01-new-on-a-template
tap 550 850
wait 100
expect library.paper is 10
expect library.template is
tap 300 1650
wait 200
expect library.sheet is templates
expect templates.selected is
tap 232 452
wait 200
expect library.template is Dots
tap 300 1650
wait 200
expect templates.selected is Dots
tap 511 452
wait 200
expect library.sheet is new
expect library.template is Lined
tap 1564 2254
wait 300
expect screen is editor
expect editor.paper_hex is 0x50f4d83c
expect editor.picture is missing
shot 02-first-page-white
waitbg
wait 300
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 03-first-page-on-it
pentap panel next
wait 400
expect editor.page is 2
expect editor.paper_hex is 0x50f4d83c
expect editor.picture is shown
pentap panel previous
wait 400
expect editor.page is 1
pentap panel library
wait 600
expect screen is library
expect library.card.1 is thumb
expect library.card.1.luma is 247
shot 04-card
quit
