# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20 GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/planner grimoire/send
# Send a PDF from the Share page (2.2, issue #175): a browser on the Wi-Fi sends "2027 Planner.pdf" as the page's
# script does; once its bytes are in, the Kindle asks "Import "2027 Planner.pdf" (4 KB)?" over the Share screen,
# the bytes wait in grimoire/.cache/share-upload.part and nothing is in grimoire/import. Import puts the PDF there
# (the toast says so, the browser is told), and back in the Library the import makes it a notebook as any PDF
# copied in, at once (Import asks the Library's scan to look as soon as it is in front). The same PDF sent again goes
# in beside it as "2027 Planner 2.pdf", never over it, and the import takes it for the PDF it is: one notebook.
# rust-only
wait 300
expect screen is library
tap 1345 95
wait 300
expect screen is share
expect share.upload is none
expect share.ask is
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
expect share.upload is asking
expect storage.exists.grimoire/.cache/share-upload.part is 1
expect storage.names.grimoire/import is .lock|README.txt
expect upload.reply.1 is
shot 01-question
# (a tap counts once the question has been on the panel a while: app_asked_seen)
wait 500
tap share ask.import
waitfor upload.reply.1 is 200 imported
expect share.ask is
expect share.upload is none
expect toast is Copied to grimoire/import.
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|2027 Planner.pdf|README.txt
shot 02-imported
# again: beside it, never over it
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
wait 500
tap share ask.import
waitfor upload.reply.2 is 200 imported
expect storage.names.grimoire/import is .lock|2027 Planner 2.pdf|2027 Planner.pdf|README.txt
tap share done
wait 300
expect screen is library
waitfor library.items is 1
waitbg
wait 300
expect library.items is 1
expect storage.names.grimoire/import is .lock|.marks|README.txt|done
shot 03-notebooks
quit
