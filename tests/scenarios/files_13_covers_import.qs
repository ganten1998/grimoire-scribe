# settings: status_mode=1
# settings: files_card=1
# settings: files_view=covers
# storage: 1
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=1 GRIMOIRE_BG_DELAY_MS=2500
# copy: tests/fixtures/files .
# Files' Covers view (2.2, issue #145) keeps what the list shows of an import, and never goes ahead of one. While a
# PDF of grimoire/import is imported, its tile in Files > Import says "Importing: page N of M" under its name, as
# the list's row does (the staying notice is off that place, as its tiles say it), and the PDF's own preview waits:
# a preview's job runs behind every slice of the import. When the import is done the file is in import/done, and
# after Look again the place shows that folder as Imported.
# rust-only
waitfor notice has Importing: page
tap library files-card
wait 300
tap library files.item.Import
waitfor files.says.1 has Importing: page
expect files.view is covers
expect files.place is grimoire/import
expect files.status.1 is importing
expect files.tiles is li
expect files.making is 1
expect notice is
shot 01-importing
waitbg
wait 300
tap library files.row.2
wait 500
expect files.row.1 is done
expect files.status.1 is imported
expect files.says.1 is Imported
shot 02-done
quit
