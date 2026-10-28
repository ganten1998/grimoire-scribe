# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# KOReader's plugin asked for a page (--annotate), but no request is in import/.annotate (lost on
# the way): nothing opens, and the toast says grimoire.log has why.
wait 300
expect screen is library
expect annotate.result is failed
expect library.items is 0
expect toast is Could not open the page KOReader sent (see grimoire.log).
quit
