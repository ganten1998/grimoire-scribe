# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: sleep_minutes=1
# env: GRIMOIRE_SLEEP_MINUTE_MS=4000
# Keys and controller buttons count as someone here, as pen and touch do: paging with them alone
# keeps the Kindle awake past "Sleep after" (one minute, which lasts 4 s here), and the power
# service hears of them at most once a minute. With no input at all, the timer still runs out.
# rust-only
wait 150
expect settings.sleep_minutes is 1
expect power.pokes is 0
# A key before any pen or touch: the power service hears of it.
key 106 0
wait 60
expect asleep is 0
expect power.pokes is 1
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 200
expect screen is editor
pentap panel next
wait 120
expect editor.pages is 2
expect editor.page is 2
# 7 s of keys only, one every half second: Left and Right, then the controller's previous and next.
key 105 0
wait 500
expect editor.page is 1
key 106 0
wait 500
expect editor.page is 2
key 314 0 1
wait 500
expect editor.page is 1
key 315 0 1
wait 500
expect editor.page is 2
key 105 0
wait 500
key 106 0
wait 500
key 314 0 1
wait 500
key 315 0 1
wait 500
key 105 0
wait 500
key 106 0
wait 500
key 314 0 1
wait 500
key 315 0 1
wait 500
key 105 0
wait 500
key 106 0
wait 500
expect asleep is 0
expect editor.page is 2
# Within the minute since the first key, so none of them told the power service again.
expect power.pokes is 1
# No input at all: asleep once the 4 s run out.
wait 6000
expect asleep is 1
# A key on the sleep screen counts for nothing.
key 105 0
wait 500
expect asleep is 1
expect editor.page is 2
expect power.pokes is 1
quit
