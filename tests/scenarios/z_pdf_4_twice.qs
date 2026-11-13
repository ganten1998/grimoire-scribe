# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/twice import
# pdfs: 0
# The same PDF dropped twice (two files, one content: the key is KOReader's partial MD5 of the bytes) makes one
# notebook; the second waits for the first's (its job ends at once, holding nothing), is taken again by the scan
# that follows, says "PDF already imported." and moves to import/done/ too.
# rust-only
waitfor toast is PDF already imported.
shot 01-twice
waitbg
wait 300
expect library.items is 1
quit
