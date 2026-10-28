# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# copy: tests/fixtures/annotate/page-12 import/.annotate
# A fresh request, but this start was not asked to open it (no GRIMOIRE_ANNOTATE: Grimoire started
# by Open Grimoire, or from the Library, after a hand-over that never reached it): deleted unopened.
wait 300
expect screen is library
expect annotate.result is stale
expect annotate.leftover is 0
expect library.items is 0
quit
