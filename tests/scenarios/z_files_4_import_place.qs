# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/files .
# The Import place: grimoire/import lists what is in it with a status on each row, and a Look again row last. A PDF
# just copied in has not held still yet: Waiting. (The row says it, so no notice is needed there.)
# rust-only
wait 300
tap library new
wait 200
tap library new.file
wait 300
tap library files.row.1
wait 1500
shot 01-import-place
expect files.place is grimoire/import
expect files.crumbs is Files > Import
expect files.rows is 2
expect files.row.1 is Waiting.pdf
expect files.kind.1 is pdf
expect files.status.1 is waiting
expect files.sub.1 is Waiting
expect files.row.2 is Look again
expect files.kind.2 is look
tap library files.row.2
wait 400
expect files.place is grimoire/import
expect files.rows is 2
tap library files.back
wait 300
expect files.place is root
quit
