# Sleep screen continuity: sleep shows the page (no toolbar), wake brings the UI back;
# pin page 1, go to page 2, sleep: the pinned page shows.
wait 200
tap 965 95
wait 100
tap 765 656
tap 1564 1934
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
pentap 1768 1674
wait 300
shot 03-display-sheet-sleep
tap 496 1298
wait 300
tap 1464 1628
wait 300
pentap 1768 1438
wait 300
pen line 300 600 1300 650 40 0.6 0.6
wait 700
sleep
wait 400
shot 04-asleep-pinned-page-1
wake
wait 400
quit
