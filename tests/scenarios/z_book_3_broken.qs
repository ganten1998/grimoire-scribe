# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_BOOK=1
# copy: tests/fixtures/book/broken import/.book
# A whole book whose second page's picture was cut short (KOReader stopped while writing it): no
# notebook is made, a toast says why, and the request and its pictures are gone.
wait 600
shot 01-toast
expect screen is library
expect annotate.book is failed
expect annotate.book_leftover is 0
expect library.items is 0
expect toast is Could not open the book KOReader sent: a page's picture does not read.
quit
