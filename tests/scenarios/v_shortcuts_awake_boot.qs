# settings: shortcuts_enabled=1
# env: GRIMOIRE_SIM_CLOCK=script
# env: GRIMOIRE_SIM_UPTIME_MS=1000
# The first input on a machine up less than a minute (one second here): the power service hears of
# it all the same, and of nothing more within the minute after it.
# rust-only
wait 150
expect power.pokes is 0
key 106 0
wait 60
expect power.pokes is 1
key 105 0
wait 60
tap 965 95
wait 100
expect power.pokes is 1
quit
