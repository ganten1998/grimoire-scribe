# Live view's pairing card: Share's Live view button turns it on (the chip shows, its code and QR
# code on the card); Done keeps it on; Share's button shows the card again while it is on; Stop
# turns it off. The Wi-Fi radio's power save is held off while it runs, and given back after.
# tests/live_check.sh holds a copy at the card (its "live_check holds here" line)
# while it pairs as a browser.
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
wait 300
expect screen is library
tap 1345 95
wait 300
expect screen is share
expect live.on is 0
shot 01-share
tap 318 2100
wait 300
# live_check holds here
expect screen is share
expect live.on is 1
expect live.pairing is 1
expect live.viewers is 0
expect power.wifi_awake is 1
shot 02-card
tap 1538 2100
wait 300
expect screen is library
expect live.on is 1
expect live.pairing is 0
expect live.chip is 1814 4 42 42
shot 03-library-chip
tap 1345 95
wait 300
tap 318 2100
wait 300
expect live.pairing is 1
expect live.on is 1
tap 318 2100
wait 300
expect screen is library
expect live.on is 0
expect live.chip is
expect toast is Live view ended.
expect power.wifi_awake is 0
shot 04-stopped
quit
