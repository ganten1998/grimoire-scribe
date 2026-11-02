# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/title-ja import/.annotate
# pdfs: 1
# A Japanese book title: its notebook keeps the title whole as its name (and its PDF's), and the
# Library draws it with DotGothic16 under its card.
# rust-only
wait 300
expect screen is editor
expect annotate.result is made
expect annotate.book_page is 3/40
pentap panel library
wait 600
expect screen is library
expect library.items is 1
expect library.card.1.name is 吾輩は猫である
shot 01-card-title
quit
