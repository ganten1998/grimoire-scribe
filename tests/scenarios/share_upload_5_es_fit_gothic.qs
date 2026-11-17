# settings: status_mode=1
# storage: 1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_UPLOAD_ASK_MS=1500
# copy: tests/fixtures/pdf/planner grimoire/send
# Send a PDF from the Share page (2.2, issue #175) in Spanish, every text checked for fit (share_upload_5_es_fit_classic
# runs the same in Classic): the Kindle's question upright and sideways, its Importar toast, and the toast of a
# question that went unanswered (GRIMOIRE_UPLOAD_ASK_MS shortens the wait).
# rust-only
wait 300
tap 1345 95
wait 300
expect screen is share
upload send send/2027 Planner.pdf
waitfor share.ask is ¿Importar «2027 Planner.pdf» (4 KB)?
shot 01-question
rotate 1
wait 600
expect share.ask is ¿Importar «2027 Planner.pdf» (4 KB)?
shot 02-question-sideways
wait 500
tap share ask.import
waitfor upload.reply.1 is 200 imported
expect toast is Se copió a grimoire/import.
shot 03-imported
rotate 0
wait 600
upload send send/2027 Planner.pdf
waitfor share.ask is ¿Importar «2027 Planner.pdf» (4 KB)?
waitfor upload.reply.2 is 408 no answer on the Kindle
waitfor share.ask is
expect toast is No se importó.
shot 04-unanswered
quit
