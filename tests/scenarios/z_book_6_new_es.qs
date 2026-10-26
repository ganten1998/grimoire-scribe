# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/book/three import/.book
# pdfs: 1
# z_book_1_new in Spanish, with the fit check: the toast saying how many pages were added fits.
wait 600
shot 01-added
expect screen is editor
expect annotate.book is done
expect toast is Se añadieron 3 páginas del libro.
pentap 1768 1566
wait 600
quit
