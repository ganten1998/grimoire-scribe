# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_PASSCODE=left-off
# copy: tests/fixtures/annotate/broken import/.annotate
# The Kindle's passcode left off by an earlier session, and a page from KOReader whose picture does
# not read: the start's one toast is the passcode's, which goes before anything (the Kindle is
# unprotected). Why the page did not open is in grimoire.log.
wait 300
expect screen is library
expect annotate.result is failed
expect toast is Your Kindle passcode was left off. Turn it back on in Kindle Settings.
quit
