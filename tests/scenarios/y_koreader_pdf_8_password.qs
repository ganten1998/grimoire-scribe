# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/pdf/locked documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book c310a5bd72674b5ab4aa992417dda973 --page 1 --pages 3
# pdfs: 0
# rust-only
# KOReader hands over by its path a PDF that needs a password (2.2, request.ini version 2: the plugin asks KOReader
# first and draws such a PDF itself, so this is the case where that check could not tell). The PDF import refuses it
# before any page is drawn, and the user reads the import's own words for it, as for the same file dropped in
# grimoire/import, not the book's general failure. No notebook is made, and the PDF stays where it is.
waitfor annotate.book is failed
wait 200
shot 01-refused
expect screen is library
expect toast is PDF needs a password.
expect annotate.book_version is 2
expect library.items is 0
expect storage.exists.documents/Locked.pdf is 1
expect annotate.book_leftover is 0
waitbg
quit
