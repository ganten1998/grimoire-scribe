# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/broken import/.annotate
# The page picture is cut short (its header reads, its data stops before the end): nothing is made,
# the request goes, and a toast says why. Grimoire starts on its Library.
wait 300
shot 01-toast
expect screen is library
expect annotate.result is failed
expect annotate.leftover is 0
expect library.items is 0
expect toast is Could not open the page KOReader sent: its picture does not read.
quit
