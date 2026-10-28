# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/book/broken import/.book
# z_book_3_broken in Spanish, with the fit check (a text cut short to fit fails the run): the toast
# that says why holds its words whole.
wait 600
shot 01-toast
expect annotate.book is failed
expect toast is No se pudo abrir el libro que envió KOReader: no se puede leer la imagen de una página.
quit
