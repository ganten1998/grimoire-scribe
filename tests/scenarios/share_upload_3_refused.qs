# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# copy: tests/fixtures/pdf/planner grimoire/send
# copy: tests/fixtures/photos grimoire/send
# Send a PDF from the Share page (2.2, issue #175), refused: with Share closed nothing answers at all; open, a send
# without the page's token, with a wrong one, from another site's page, or of a file that is no PDF is answered
# with its refusal, and the Kindle asks nothing and keeps nothing. The page's own send is still taken after them.
# rust-only
wait 300
expect screen is library
upload send send/2027 Planner.pdf
waitfor upload.reply.1 has error
tap 1345 95
wait 300
expect screen is share
upload notoken send/2027 Planner.pdf
waitfor upload.reply.2 is 401 token required
upload badtoken send/2027 Planner.pdf
waitfor upload.reply.3 is 401 wrong token
upload othersite send/2027 Planner.pdf
waitfor upload.reply.4 is 403 another site
upload send send/IMG_0002.png
waitfor upload.reply.5 is 415 not a PDF
wait 300
expect share.ask is
expect share.upload is none
expect toast is
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|README.txt
shot 01-nothing-asked
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
wait 500
tap share ask.cancel
waitfor upload.reply.6 is 410 declined on the Kindle
quit
