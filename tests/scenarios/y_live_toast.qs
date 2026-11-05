# rust-only
# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/update-failed
# copy: tests/fixtures/gestures/actions .
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# Toasts in Live view (2.0.0-beta.2; the owner, 2026-11-05: "Enable toasts on the live view display. they
# don't show up now."): the toast is a layer of the stream (live.toast: what a browser is sent), in the same
# place as on the panel and, drawn with its alpha over the screen as a browser draws it, as the panel shows
# it (live.toast.stale 0: no pixel more than a unit of gray off). A toast over the pairing card is never in
# it, and one still up as the card closes comes then; it moves with the floor
# (the New sheet) and with a turn, follows Invert colors as the panel's picture does, is replaced by the
# next toast, and goes when its time is up, when the Kindle sleeps (and stays gone at the wake), and when
# it went while the card was up. (The fixture's notice of an update that did not install is up for 6 s
# from the start; the keys are the actions fixture's: 6 Invert colors, 7 and 8 the light.)
clock 1700000000
wait 300
expect toast is Last update failed. See grimoire.log.
expect toast.rect is 476 2250 907 110
# Live view off: no layer
expect live.toast is
tap 1345 95
wait 300
tap 318 2100
wait 300
expect live.pairing is 1
# the toast is up over the card, and not in the frame
expect toast is Last update failed. See grimoire.log.
expect live.toast is
tap 1538 2100
wait 300
expect screen is library
expect live.pairing is 0
# the card gone and the toast still up: it comes now
expect live.toast is 476 2250 907 110
expect live.toast.stale is 0
# above the New sheet's buttons, and back
tap 965 95
wait 100
expect library.sheet is new
expect toast.rect is 476 2050 907 110
expect live.toast is 476 2050 907 110
expect live.toast.stale is 0
tap 300 2250
wait 100
expect library.sheet is none
expect live.toast is 476 2250 907 110
# sideways, and back
rotate 1
wait 400
expect rotation is 1
expect toast.rect is 786 1630 907 110
expect live.toast is 786 1630 907 110
expect live.toast.stale is 0
rotate 0
wait 400
expect live.toast is 476 2250 907 110
expect live.toast.stale is 0
# another toast in its place, inverted (the layer as drawn, the panel inverted), then its time up
key 7 0
wait 100
expect toast is Light 11 of 24
expect toast.rect is 739 2250 382 110
expect live.toast is 739 2250 382 110
expect live.toast.stale is 0
key 6 0
wait 300
expect settings.invert is 1
expect live.toast is 739 2250 382 110
expect live.toast.stale is 0
key 6 0
wait 300
expect settings.invert is 0
wait 2300
expect toast is
expect live.toast is
# asleep: gone at once, and none at the wake
key 8 0
wait 100
expect toast is Light 12 of 24
expect live.toast is 733 2250 394 110
sleep
wait 200
expect asleep is 1
expect live.toast is
wake
wait 300
expect asleep is 0
expect toast is
expect live.toast is
# a toast raised over the card is not in the frame, nor after it once it went
tap 1345 95
wait 300
tap 318 2100
wait 300
expect live.pairing is 1
hid attach pad Pad
wait 100
expect toast is Controller connected
expect live.toast is
wait 1800
expect toast is
tap 1538 2100
wait 300
expect live.pairing is 0
expect live.toast is
quit
