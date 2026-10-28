# settings: status_mode=1
# Sleep screen continuity: sleep shows the page (no toolbar), wake brings the UI back;
# pin page 1, go to page 2, sleep: the pinned page shows.
# rust-only
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen curve 300 520 50 70 0.7 4
pen line 300 760 1400 780 60 0.4 0.9
wait 700
expect editor.strokes is 2
expect asleep is 0
sleep
wait 400
shot 01-asleep-on-page
expect asleep is 1
wake
wait 400
shot 02-awake
expect asleep is 0
expect screen is editor
pentap panel display
wait 300
shot 03-display-sheet-sleep
expect editor.modal is display
expect settings.sleep_mode is 0
tap 496 1532
expect toast is This page is now the sleep screen
expect settings.sleep_mode is 1
wait 300
tap 1464 1862
wait 300
expect editor.modal is none
pentap panel next
wait 300
expect editor.page is 2
expect editor.pages is 2
pen line 300 600 1300 650 40 0.6 0.6
wait 700
sleep
wait 400
shot 04-asleep-pinned-page-1
expect asleep is 1
wake
wait 400
expect asleep is 0
# power button with no announcement from the power service: Grimoire asks for sleep itself
power
wait 3200
shot 05-power-fallback-asleep
expect asleep is 1
# a tap on the sleep screen must not draw Grimoire over it
pentap 900 1200
wait 300
shot 06-tap-while-asleep-still-asleep
expect asleep is 1
expect editor.strokes is 1
# power pressed while asleep, and no outOfScreenSaver ever comes: wake after 3 s anyway
power
wait 3300
shot 07-power-fallback-awake
expect asleep is 0
# an outOfScreenSaver while already awake still repaints (no lost wake)
wake
wait 300
shot 08-wake-while-awake
expect asleep is 0
expect screen is editor
expect editor.page is 2
quit
