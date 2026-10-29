# settings: status_mode=1
# settings: status_on=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/status-keys .
# env: TZ=UTC
# rust-only
# The clock corner takes a SLOW, deliberate press as surely as a light tap (the owner on a Kindle: "the taps being
# inconsistent to bring it up or dismiss it at all are a killer ... spamming taps really fast can more consistently
# summon it"). A pressed fingertip flattens and its centroid drifts, so the old rule (the finger stayed within 5 mm
# of where it landed, up within 600 ms) lost the slow ones. Now: down in the zone, lifted in the zone, within
# 800 ms, whatever the path. Every corner, each with a slow press that drifts across most of the zone.
wait 3800
clock 1700000000
battery 87
wait 100
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect settings.status_on is 0
# corner 0: zone at 0 0
expect settings.status_corner is 0
expect status.zone is 0 0 165 165
# a slow press (700 ms) whose centroid drifts 9 mm and ends in the zone: a tap
touch down 70 70
wait 300
touch move 105 105
wait 300
touch move 150 150
wait 100
touch up 150 150
wait 300
expect settings.status_on is 1
# a light fast tap
tap 70 70
wait 300
expect settings.status_on is 0
# a press of 780 ms, lifting where it landed
touch down 70 70
wait 780
touch up 72 72
wait 300
expect settings.status_on is 1
# a drag out of the zone is not a tap
touch down 70 70
touch move 370 370
touch up 370 370
wait 300
expect settings.status_on is 1
# a press longer than 800 ms is the page's
touch down 70 70
wait 900
touch up 70 70
wait 300
expect settings.status_on is 1
key 46 0
wait 300
key 31 0
wait 300
expect settings.status_on is 0
# corner 1: zone at 1695 0
expect settings.status_corner is 1
expect status.zone is 1695 0 165 165
# a slow press (700 ms) whose centroid drifts 9 mm and ends in the zone: a tap
touch down 1765 70
wait 300
touch move 1800 105
wait 300
touch move 1845 150
wait 100
touch up 1845 150
wait 300
expect settings.status_on is 1
# a light fast tap
tap 1765 70
wait 300
expect settings.status_on is 0
# a press of 780 ms, lifting where it landed
touch down 1765 70
wait 780
touch up 1767 72
wait 300
expect settings.status_on is 1
# a drag out of the zone is not a tap
touch down 1765 70
touch move 1465 370
touch up 1465 370
wait 300
expect settings.status_on is 1
# a press longer than 800 ms is the page's
touch down 1765 70
wait 900
touch up 1765 70
wait 300
expect settings.status_on is 1
key 46 0
wait 300
key 31 0
wait 300
expect settings.status_on is 0
# corner 2: zone at 1695 2315
expect settings.status_corner is 2
expect status.zone is 1695 2315 165 165
# a slow press (700 ms) whose centroid drifts 9 mm and ends in the zone: a tap
touch down 1765 2385
wait 300
touch move 1800 2420
wait 300
touch move 1845 2465
wait 100
touch up 1845 2465
wait 300
expect settings.status_on is 1
# a light fast tap
tap 1765 2385
wait 300
expect settings.status_on is 0
# a press of 780 ms, lifting where it landed
touch down 1765 2385
wait 780
touch up 1767 2387
wait 300
expect settings.status_on is 1
# a drag out of the zone is not a tap
touch down 1765 2385
touch move 1465 2085
touch up 1465 2085
wait 300
expect settings.status_on is 1
# a press longer than 800 ms is the page's
touch down 1765 2385
wait 900
touch up 1765 2385
wait 300
expect settings.status_on is 1
key 46 0
wait 300
key 31 0
wait 300
expect settings.status_on is 0
# corner 3: zone at 0 2315
expect settings.status_corner is 3
expect status.zone is 0 2315 165 165
# a slow press (700 ms) whose centroid drifts 9 mm and ends in the zone: a tap
touch down 70 2385
wait 300
touch move 105 2420
wait 300
touch move 150 2465
wait 100
touch up 150 2465
wait 300
expect settings.status_on is 1
# a light fast tap
tap 70 2385
wait 300
expect settings.status_on is 0
# a press of 780 ms, lifting where it landed
touch down 70 2385
wait 780
touch up 72 2387
wait 300
expect settings.status_on is 1
# a drag out of the zone is not a tap
touch down 70 2385
touch move 370 2085
touch up 370 2085
wait 300
expect settings.status_on is 1
# a press longer than 800 ms is the page's
touch down 70 2385
wait 900
touch up 70 2385
wait 300
expect settings.status_on is 1
key 46 0
wait 300
key 31 0
wait 300
expect settings.status_on is 0
quit
