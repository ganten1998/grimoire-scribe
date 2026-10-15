# Sleep screen continuity: sleep shows the page (no toolbar), wake brings the UI back;
# pin page 1, go to page 2, sleep: the pinned page shows.
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
pen curve 300 520 50 70 0.7 4
pen line 300 760 1400 780 60 0.4 0.9
wait 700
sleep
wait 400
shot 01-asleep-on-page
wake
wait 400
shot 02-awake
pentap 1768 1350
wait 300
shot 03-display-sheet-sleep
tap 496 1298
wait 300
tap 1464 1628
wait 300
pentap 1768 1114
wait 300
pen line 300 600 1300 650 40 0.6 0.6
wait 700
sleep
wait 400
shot 04-asleep-pinned-page-1
wake
wait 400
# power button with no announcement from the power service: Quire asks for sleep itself
power
wait 3200
shot 05-power-fallback-asleep
# a tap on the sleep screen must not draw Quire over it
pentap 900 1200
wait 300
shot 06-tap-while-asleep-still-asleep
# power pressed while asleep, and no outOfScreenSaver ever comes: wake after 3 s anyway
power
wait 3300
shot 07-power-fallback-awake
# an outOfScreenSaver while already awake still repaints (no lost wake)
wake
wait 300
shot 08-wake-while-awake
quit
