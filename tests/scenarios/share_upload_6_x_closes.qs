# settings: status_mode=1
# settings: shortcuts_enabled=1
# storage: 1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# env: GRIMOIRE_UPLOAD_QUIET_MS=0
# copy: tests/fixtures/pdf/planner grimoire/send
# Send a PDF from the Share page (2.2, issue #175): Share's X stays live while the Kindle asks. It is the question's
# Cancel, and Share closes, even right as the question appears (a refusal waits for nothing). Here the next send comes
# at once after each answer (the quiet time made 0), as from a sender who sends again and again: its question is up
# before the user's next tap, and X still closes Share. Esc does the same. Nothing is kept.
# rust-only
wait 300
tap 1345 95
wait 300
expect screen is share
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
expect share.upload is asking
wait 500
tap share ask.cancel
waitfor upload.reply.1 is 200 declined on the Kindle
# the next send at once: its question is up again, and X closes Share at once
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
tap share done
wait 300
expect screen is library
waitfor upload.reply.2 is 200 declined on the Kindle
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|README.txt
shot 01-closed
# Share again: a question, its Cancel, the next at once, and Esc
tap 1345 95
wait 300
expect screen is share
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
wait 500
tap share ask.cancel
waitfor upload.reply.3 is 200 declined on the Kindle
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
key 1 0
wait 300
expect screen is library
waitfor upload.reply.4 is 200 declined on the Kindle
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|README.txt
wait 600
expect library.items is 0
quit
