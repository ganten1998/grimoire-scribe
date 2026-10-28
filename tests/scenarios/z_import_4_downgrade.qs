# An imported notebook that 1.0.0-beta wrote on: it saved the page as version 1 Blank (the ink
# kept) and drew its thumbnail on blank paper, and bg/pages.ini still lists the page with its
# picture. Opened here, the page has its picture again (and is saved so); closed, the Library
# card is drawn again with the picture, and the notebook's PDF is written with it.
# copy: tests/fixtures/scen-downgrade .
# pdfs: 1
# rust-only
wait 500
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
expect library.card.1.luma is 255
tap 326 608
wait 600
shot 01-page
expect screen is editor
expect editor.pages is 1
expect editor.paper is 1358954497
expect editor.strokes is 1
pentap panel library
wait 600
shot 02-card
expect screen is library
expect library.card.1 is thumb
expect library.card.1.luma is 232
quit
