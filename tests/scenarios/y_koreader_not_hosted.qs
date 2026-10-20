# env: GRIMOIRE_HOST=
# Started from the Kindle's Library: no host (GRIMOIRE_HOST is set empty here, so a caller's
# never reaches the run). Sleep mode 2 stays the Kindle's own screensaver, which Grimoire leaves
# to the Kindle UI (the simulator draws nothing of it): asleep.mode says 2. y_koreader_hosted holds
# the other side: hosted by KOReader, mode 2 shows the page instead.
wait 200
expect host is
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen line 300 1700 1400 1720 60 0.4 0.9
wait 700
expect editor.strokes is 1
pentap 1768 1350
wait 300
expect editor.modal is display
tap 800 1172
wait 200
tap 800 1172
wait 200
expect settings.sleep_mode is 2
tap 1464 1628
wait 300
expect editor.modal is none
expect asleep.mode is
sleep
wait 400
expect asleep is 1
expect asleep.mode is 2
wake
wait 400
expect asleep is 0
expect asleep.mode is
expect screen is editor
quit
