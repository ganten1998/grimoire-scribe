# settings: status_mode=1
# settings: shortcuts_enabled=1
# storage: 1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20 GRIMOIRE_IMPORT_STABLE=fast
# env: GRIMOIRE_UPLOAD_QUIET_MS=1000
# copy: tests/fixtures/pdf/planner grimoire/send
# Send a PDF from the Share page (2.2, issue #175), cancelled: Cancel on the Kindle's question keeps nothing (no
# temporary file, nothing in grimoire/import, no toast) and tells the browser; Esc on the next question is its
# Cancel and closes Share, as Esc does without one (the keys on); a question turned with the Kindle is drawn at its new
# place and still asks; sleep ends one, and
# the Library has no notebook of any of them. After each send not imported the Kindle takes no other for a while
# (shortened to 1 s here; share_upload_4_twice holds the 5 s), so each next send waits past it.
# rust-only
wait 300
tap 1345 95
wait 300
expect screen is share
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
shot 01-question
wait 500
tap share ask.cancel
waitfor upload.reply.1 is 200 declined on the Kindle
expect share.ask is
expect share.upload is none
expect toast is
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|README.txt
shot 02-cancelled
# Esc is Cancel, and Share closes
wait 1100
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
wait 500
key 1 0
waitfor upload.reply.2 is 200 declined on the Kindle
wait 300
expect screen is library
expect storage.exists.grimoire/.cache/share-upload.part is 0
tap 1345 95
wait 300
expect screen is share
expect share.ask is
# turned: drawn at its new place, still asked, answered there
wait 1100
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
rotate 1
wait 600
expect share.ask is Import "2027 Planner.pdf" (4 KB)?
expect upload.samecode.3 is 1
expect share.upload is asking
shot 03-turned
wait 500
tap share ask.cancel
waitfor upload.reply.3 is 200 declined on the Kindle
rotate 0
wait 1100
# sleep ends the question, and the browser is told; the wake shows Share without it
upload send send/2027 Planner.pdf
waitfor share.ask is Import "2027 Planner.pdf" (4 KB)?
sleep
wait 300
waitfor upload.reply.4 is 200 the Kindle is asleep
expect storage.exists.grimoire/.cache/share-upload.part is 0
wake
wait 600
expect screen is share
expect share.ask is
expect share.upload is none
tap share done
wait 300
expect screen is library
expect storage.exists.grimoire/.cache/share-upload.part is 0
expect storage.names.grimoire/import is .lock|README.txt
wait 600
expect library.items is 0
quit
