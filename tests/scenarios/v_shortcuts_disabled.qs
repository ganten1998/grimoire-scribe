# settings: sleep_minutes=1
# env: GRIMOIRE_SLEEP_MINUTE_MS=4000
wait 100
expect settings.shortcuts_enabled is 0
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 120
key 7 0
wait 60
expect editor.tool is pencil
expect shortcuts.code is 0
# With shortcuts off, keys count for nothing: "Sleep after" (one minute, which lasts 4 s here)
# runs out while they are pressed.
expect asleep is 0
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
key 7 0
wait 500
expect asleep is 1
quit
