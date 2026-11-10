# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=1500
# copy: tests/fixtures/files .
# While a PDF in grimoire/import is imported, the Import place's row says "Importing: page %d of %d" and the
# staying notice is off that place (it shows on the Library and on the picker's other screens); when it is done the
# file is in import/done and the place shows that folder as Imported, after Look again.
# rust-only
waitfor notice has Importing: page
tap library new
wait 200
tap library new.file
wait 300
expect notice.key is import
tap library files.row.1
wait 1500
shot 01-importing
expect files.status.1 is importing
expect files.sub.1 has Importing: page
expect notice is
waitbg
wait 300
tap library files.row.2
wait 500
shot 02-done
expect files.place is grimoire/import
expect files.row.1 is done
expect files.status.1 is imported
expect files.sub.1 is Imported
quit
