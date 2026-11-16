# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1 GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=600
# copy: tests/fixtures/pdf/tabs documents
# generate: tests/book_v2_request.py --dir documents --data grimoire --book 1fba66e74df2ca9847e3725e1571df47 --page 3 --pages 4
# pdfs: 1
# rust-only
# KOReader hands a PDF over by its path (2.2, request.ini version 2) and the Kindle goes to sleep before its import
# ends (each slice held back 0.6 s here, one page a slice). The import goes on behind the sleep screen, and nothing
# opens there: its end is taken at the wake, when the Library's cards are in front again, and the notebook then opens
# at the reader's page, book page 3.
wait 300
expect screen is library
expect annotate.book is running
sleep
wait 300
expect asleep is 1
waitbg
wait 800
expect annotate.book is running
expect screen is library
wake
waitfor screen is editor
expect asleep is 0
expect annotate.book is done
expect toast is Added 4 book pages.
expect editor.pages is 4
expect editor.page is 3
expect annotate.book_page is 3/4
pentap panel library
wait 600
waitbg
wait 300
expect library.items is 1
quit
