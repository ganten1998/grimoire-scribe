# root: z_annotate_3_order
# copy: tests/fixtures/book/merge import/.book
# A book request left by a hand-over that never reached Grimoire, found by a start from the Library
# (no GRIMOIRE_BOOK): deleted unopened, the book's notebook as it was.
wait 400
expect screen is library
expect annotate.book is stale
expect annotate.book_leftover is 0
expect library.items is 1
quit
