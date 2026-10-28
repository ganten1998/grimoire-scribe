# settings: status_mode=1
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/page-12 import/.annotate
# GRIMOIRE_ANNOTATE=1, but Grimoire was not started by KOReader (no GRIMOIRE_HOST: kindle/app/grimoire.sh
# passes the variable on for a hosted start only, so this is one left over from elsewhere). A fresh
# request is not opened all the same: deleted unopened, nothing made.
wait 300
expect screen is library
expect annotate.result is stale
expect annotate.leftover is 0
expect library.items is 0
quit
