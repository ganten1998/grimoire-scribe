# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/annotate/sideways import/.annotate
# pdfs: 1
# z_annotate_8_sideways's page, looked at and left at once, while its picture is still fitted in
# the background (each job waits 1.5 s here): the editor makes the Library's card from the page as
# it shows it, white. The job that fits the picture draws the card again with it, and the Library
# reads the card again once the job is done.
# rust-only
wait 300
expect screen is editor
expect annotate.result is made
expect annotate.luma is 255
pentap panel library
wait 600
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
expect library.card.1.luma is 255
expect library.cards_redrawn is 0
shot 01-card-white-while-fitted
waitbg
wait 300
expect library.card.1 is thumb
expect library.card.1.luma not 255
expect library.cards_redrawn is 1
shot 02-card-with-the-page
quit
