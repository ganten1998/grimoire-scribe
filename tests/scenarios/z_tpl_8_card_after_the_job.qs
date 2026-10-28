# A notebook made on a template and written on at once, then left before the background has made
# the template's picture for it (the jobs are slowed here, as a big picture is on the Kindle): its
# Library card is drawn on white paper first, and again with the picture once the job is done.
# Its PDF, written after that job, has the picture too.
# copy: tests/fixtures/scen-templates templates
# env: GRIMOIRE_BG_DELAY_MS=3000
# pdfs: 1
# rust-only
wait 300
tap 965 95
wait 100
tap 300 1650
wait 200
expect library.sheet is templates
# (the previews: a slowed job, waited for)
waitbg
wait 300
expect templates.ready is 2
tap 511 452
wait 200
expect library.template is Lined
tap 1564 2254
wait 300
expect screen is editor
expect editor.picture is missing
pen line 300 640 1300 700 40 0.6 0.6
wait 300
expect editor.strokes is 1
pentap panel library
wait 600
expect screen is library
expect library.card.1 is thumb
expect library.card.1.luma is 255
shot 01-card-white-paper
expect library.cards_redrawn is 0
# (the template's picture: a slowed job, waited for)
waitbg
wait 300
expect library.card.1 is thumb
expect library.card.1.luma is 247
expect library.cards_redrawn is 1
shot 02-card-with-the-picture
tap 326 608
wait 600
expect editor.picture is shown
expect editor.strokes is 1
quit
