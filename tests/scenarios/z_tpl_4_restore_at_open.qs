# A notebook 1.0.0-beta wrote on: it saved page 1 (an imported page) as version 1 Blank and drew
# the Library's card on blank paper, and bg/pages.ini still lists page 1 with its picture. Opened
# here on page 2, page 1 gets its picture back at once, though it is not visited: closed, the card
# is drawn again with the picture, and the notebook's PDF is written with it.
# copy: tests/fixtures/scen-tpl-restore .
# pdfs: 1
wait 500
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
expect library.card.1.luma is 255
tap 326 608
wait 600
expect screen is editor
expect editor.pages is 2
expect editor.page is 2
expect editor.paper is 1
expect editor.unsaved is 0
pentap 1768 1566
wait 600
expect screen is library
expect library.card.1 is thumb
expect library.card.1.luma is 236
shot 01-card-drawn-again
tap 326 608
wait 600
pentap 1768 938
wait 400
expect editor.page is 1
expect editor.paper is 1358954497
expect editor.strokes is 1
shot 02-page-1-on-its-picture
quit
