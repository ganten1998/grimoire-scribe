# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# copy: tests/fixtures/annotate/broken import/.annotate
# z_annotate_6_broken in Spanish, with the fit check (a text cut short to fit fails the run): the
# toast that says why holds its words whole.
wait 300
shot 01-toast-es
expect language is es
expect screen is library
expect annotate.result is failed
expect library.items is 0
expect toast is No se pudo abrir la página que envió KOReader: no se puede leer su imagen.
quit
