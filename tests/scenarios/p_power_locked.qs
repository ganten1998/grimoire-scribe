# A Kindle that stays locked after the power button (its passcode waiting): a press with no wake
# leaves Grimoire on its sleep screen; a second one in a row saves and leaves Grimoire, so the
# Kindle gets its pen and touch back even where the 4 s hold cannot be seen.
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
pen line 300 760 1400 780 60 0.4 0.9
wait 700
sleep
wait 300
powerd screenSaver
power
wait 3300
expect asleep is 1
shot 01-locked-first-press-stays-asleep
# The Kindle wakes after all (outOfScreenSaver): it was not locked, so the count starts again and
# the next press with no wake only keeps Grimoire asleep.
wake
wait 400
expect asleep is 0
sleep
wait 300
power
wait 3300
expect asleep is 1
shot 02-after-a-wake-one-press-stays-asleep
# The second press in a row that brings no wake: Grimoire saves and leaves, and the run ends here.
power
wait 3300
expect running is 0
