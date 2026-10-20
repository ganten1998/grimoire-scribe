# A folder of page pictures in grimoire/import/ (copied in over USB while Grimoire was closed)
# becomes a notebook at startup, in the background: a toast says so and the Library shows it. Its
# pages are the pictures in the natural order of their names (page-1, page-2, page-10; notes.txt
# is no picture), each a document page, and a page added after the last one is blank. A template
# copied into grimoire/templates/ waits there for the Templates sheet.
# copy: tests/fixtures/scen-import import
# copy: tests/fixtures/scen-templates templates
# pdfs: 1
# The import takes about a second on a quiet machine and longer on a busy one: waitbg waits for it
# (and for the templates' previews) to end, which puts its toast up for 8 s.
waitbg
wait 300
shot 01-library-imported
expect toast is Imported a notebook of 3 pages.
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
expect library.card.1.luma is 245
tap 326 608
wait 600
shot 02-page-1
expect screen is editor
expect editor.pages is 3
expect editor.page is 1
expect editor.paper is 1370650129
pentap 1768 1114
wait 400
shot 03-page-2-colour-as-gray
expect editor.page is 2
expect editor.paper is 1372561192
pentap 1768 1114
wait 400
expect editor.page is 3
expect editor.paper is 1366081652
# Ink over the picture, then the eraser: the region it took the stroke from shows the picture again.
pen line 300 850 1300 880 40 0.6 0.6
wait 300
shot 04-page-10-ink
expect editor.strokes is 1
eraser line 300 865 1300 865 30
wait 600
shot 05-page-10-erased
expect editor.strokes is 0
# The next page: the notebook's own paper (blank), not page 10's picture.
pentap 1768 1114
wait 400
shot 06-new-page-blank
expect editor.pages is 4
expect editor.page is 4
expect editor.paper is 4
pentap 1768 1566
wait 600
expect screen is library
quit
