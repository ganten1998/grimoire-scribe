# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/tabs documents/Planner.sdr
# generate: tests/book_v2_request.py --dir documents/Planner.sdr --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 0
# rust-only
# A version 2 request that names a PDF where Grimoire does not read (2.2: here inside a .sdr folder, KOReader's own;
# the plugin sends such a PDF by version 1, so this is a request it would not write). The storage walk refuses the
# path at the start, before any import: the book fails as a book does, with the book's own words, nothing is queued
# and no notebook is made.
wait 400
shot 01-refused
expect screen is library
expect annotate.book is failed
expect annotate.book_version is 2
expect toast is Couldn't open book from KOReader (see grimoire.log).
expect notice is
expect bg.busy is 0
expect library.items is 0
expect annotate.book_leftover is 0
quit
