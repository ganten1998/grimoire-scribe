# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# env: GRIMOIRE_PASSCODE=paused
# copy: tests/fixtures/annotate/broken import/.annotate
# The passcode paused for the first time (told once, at a start), and a page from KOReader whose
# picture does not read: the toast says what became of the page, which the user just asked for.
# The passcode's notice waits for a later start (z_annotate_h_paused_later), never lost.
wait 300
expect annotate.result is failed
expect toast is Could not open the page KOReader sent: its picture does not read.
quit
