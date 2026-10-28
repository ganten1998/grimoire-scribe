# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/title-ja import/.annotate
# pdfs: 1
# A book whose title Grimoire's fonts draw none of (Japanese): its notebook keeps the title whole as
# its name (and its PDF's), and the Library, which cannot draw it, shows Untitled under its card.
# rust-only
wait 300
expect screen is editor
expect annotate.result is made
expect annotate.book_page is 3/40
pentap panel library
wait 600
expect screen is library
expect library.items is 1
expect library.card.1.name is Untitled
shot 01-card-untitled
quit
