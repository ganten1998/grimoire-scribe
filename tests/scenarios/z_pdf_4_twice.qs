# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/twice import
# pdfs: 0
# The same PDF dropped twice (two files, one content: the key is KOReader's partial MD5 of the bytes) makes one
# notebook; the second says "PDF already imported." and moves to import/done/ too.
# rust-only
waitbg
wait 300
shot 01-twice
expect toast is PDF already imported.
expect library.items is 1
quit
