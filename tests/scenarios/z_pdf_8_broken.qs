# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/junk import
# pdfs: 0
# A file that is no PDF (it starts and ends as one does) fails whole: "Couldn't import PDF (see grimoire.log).",
# no notebook, the file left in import/.
# rust-only
waitbg
wait 300
shot 01-failed
expect toast is Couldn't import PDF (see grimoire.log).
expect library.items is 0
quit
