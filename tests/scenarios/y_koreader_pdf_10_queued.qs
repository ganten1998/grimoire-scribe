# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_BG_DELAY_MS=600
# copy: tests/fixtures/pdf/colour grimoire/import
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 1
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2) while another import is ahead of it: a PDF
# dropped in grimoire/import, which the start's scan queued first. The book's PDF queues behind it as a picked PDF
# does, and that is said at once ("Another import is still going. This one starts after it."); nothing is held
# meanwhile. Both become notebooks, and with the Library's cards still in front the book's opens at the reader's
# page, book page 3.
wait 200
shot 01-waits
expect screen is library
expect annotate.book is running
expect toast is Another import is still going. This one starts after it.
tap library new
wait 300
expect library.sheet is new
tap library done
wait 300
expect library.sheet is none
waitfor screen is editor
expect annotate.book is done
expect editor.pages is 4
expect editor.page is 3
expect annotate.book_page is 3/4
pentap panel library
wait 600
waitbg
wait 300
expect library.items is 2
quit
