# settings: status_mode=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# KOReader asked for a page at a start whose home is inside Files (2.2, issue #160), but no request came: no
# notebook opens, so home does not wait for one. Files opens at home as at a plain start, under the hand-over's own
# toast.
# rust-only
wait 300
expect screen is library
expect annotate.result is failed
expect files.open is 1
expect files.place is documents/Sorted
expect files.at_home is 1
expect toast is Couldn't open page from KOReader (see grimoire.log).
quit
