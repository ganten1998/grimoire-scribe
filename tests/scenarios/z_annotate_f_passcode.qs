# rust-only
# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_PASSCODE=left-off
# copy: tests/fixtures/annotate/broken import/.annotate
# The Kindle's passcode left off by an earlier session, and a page from KOReader whose picture does
# not read: the start's one toast says why the page did not open. The passcode note is grimoire.log's
# alone since 2.0.0-beta.2 (the owner's toast pass), so it no longer takes that toast's place.
wait 300
expect screen is library
expect annotate.result is failed
expect toast is Couldn't open page: its image is unreadable.
quit
