# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-12 import/.annotate
# pdfs: 1
# KOReader's "Annotate this page in Grimoire" on page 12 of a 96-page PDF: its plugin wrote the page
# picture and a request into grimoire/import/.annotate/ and started Grimoire hosted, asked to open
# it (kindle/app/grimoire.sh sets GRIMOIRE_ANNOTATE=1). No notebook is the book's yet: one is made,
# named after its title, its one page a document page on that picture (KOReader drew it at the
# Scribe's page size, so it is the page's picture as it is, shown at once), and the editor opens on
# it. The request is gone. Ink on it, then Back to KOReader from the Library saves it all.
wait 300
shot 01-the-page
expect screen is editor
expect annotate.result is made
expect annotate.leftover is 0
expect editor.pages is 1
expect editor.page is 1
expect annotate.role is document
expect annotate.book_page is 12/96
expect annotate.luma not 255
expect editor.strokes is 0
pen line 300 900 1300 950 40 0.6 0.6
wait 700
expect editor.strokes is 1
pentap 1768 1566
wait 600
shot 02-library
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
tap 1750 95
expect running is 0
