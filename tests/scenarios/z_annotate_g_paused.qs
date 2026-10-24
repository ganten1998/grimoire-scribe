# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_PASSCODE=enabled
# copy: tests/fixtures/annotate/broken import/.annotate
# Native PIN remains enabled; an annotation failure still gets its own startup notice.
wait 300
expect annotate.result is failed
expect toast is Could not open the page KOReader sent: its picture does not read.
quit
