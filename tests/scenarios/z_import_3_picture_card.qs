# A notebook whose own paper is one of its pictures and that has no thumbnail yet (as a notebook
# made on a template will be): its Library card shows that picture (library.card.1 is "picture",
# with the grid's mean, not white paper's 255), never another paper. The picture was made for a
# page of another size than the notebook's, so it is fitted to the page as it is drawn. Opened,
# its one page (no page file yet) is on that paper, and takes ink; closed, the card is the page's
# thumbnail.
# copy: tests/fixtures/scen-picture-card .
# pdfs: 1
wait 500
shot 01-card
expect screen is library
expect library.items is 1
expect library.card.1 is picture
expect library.card.1.luma is 245
tap 326 608
wait 600
shot 02-page
expect screen is editor
expect editor.pages is 1
expect editor.paper is 1342177281
pen line 300 900 1300 950 40 0.6 0.6
wait 300
expect editor.strokes is 1
pentap 1768 1566
wait 600
shot 03-card-thumbnail
expect screen is library
expect library.card.1 is thumb
expect library.card.1.luma is 247
quit
