# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/stale import/.annotate
# A request written long ago (time= of 2001, more than five minutes away): not opened, whatever
# asked for it. One log line, the folder emptied, nothing made, and Grimoire starts on its Library.
wait 300
expect screen is library
expect annotate.result is stale
expect annotate.leftover is 0
expect library.items is 0
expect toast is
quit
