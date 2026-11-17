# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# copy: tests/fixtures/pdf/planner grimoire/send
# Send a PDF from the Share page (2.2, issue #175), twice at once: a second send while the Kindle asks about the
# first ends both, so one sender can never take the other's place under the question. Both browsers are told, the
# question goes with "Not imported.", and nothing is kept; the next send is asked about as usual.
# rust-only
wait 300
tap 1345 95
wait 300
expect screen is share
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
upload send send/2027 Planner.pdf
waitfor upload.reply.2 is 409 another PDF is being sent
waitfor upload.reply.1 is 409 another PDF was sent
waitfor share.ask is
expect share.upload is none
expect toast is Not imported.
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|README.txt
shot 01-neither
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
wait 500
tap share ask.import
waitfor upload.reply.3 is 200 imported
expect storage.names.grimoire/import is .lock|2027 Planner.pdf|README.txt
quit
