# settings: status_mode=1
# settings: home=notebooks/Nowhere
# storage: 1
# generate: tests/files_home_tree.py
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# A home that is gone at a start KOReader asked a page of, which did not come (2.2, issue #160): the start says
# "Home folder not found." as it goes home, and that toast stays: the hand-over's own toast ("Couldn't open page
# from KOReader", which the log explains) is not put over it, as it would say nothing of why the main menu is not
# at home. Home is Notebooks again.
# rust-only
wait 300
expect screen is library
expect annotate.result is failed
expect library.folder is
expect toast is Home folder not found.
expect settings.home is
quit
