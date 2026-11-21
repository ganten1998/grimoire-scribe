# settings: status_mode=1
# Live view's pairing card: Share's Live view button turns it on (the chip shows, its code and QR
# code on the card); Done keeps it on; Share opens the card directly while it is on; Stop turns it
# off. tests/live_check.sh holds a copy at the card (its "live_check holds here" line)
# while it pairs as a browser.
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
wait 300
expect screen is library
tap library add
tap library add.share
wait 300
expect screen is share
expect live.on is 0
shot 01-share
tap share live
wait 300
# live_check holds here
expect screen is share
expect live.on is 1
expect live.pairing is 1
expect live.viewers is 0
shot 02-card
tap share done
wait 300
expect screen is library
expect live.on is 1
expect live.pairing is 0
expect live.chip is 1814 4 42 42
shot 03-library-chip
tap library add
tap library add.share
wait 300
expect screen is share
expect live.pairing is 1
expect live.on is 1
tap share stop
wait 300
expect screen is library
expect live.on is 0
expect live.chip is
expect toast is Live view ended.
shot 04-stopped
quit
