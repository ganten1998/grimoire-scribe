# The power button's ways out of every stuck state: a sleep announcement Grimoire missed, a Kindle
# still locked after a press, the side button while asleep, a wake announced while the key is
# down, and the 4 s hold that saves and leaves Grimoire.
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
pen curve 300 520 50 70 0.7 4
pen line 300 760 1400 780 60 0.4 0.9
wait 700
shot 01-ink
# A short press, no goingToScreenSaver, and the power service already in its screensaver: the
# announcement was missed, so Grimoire shows its own sleep screen and sends no powerButton (it
# would wake the Kindle: the simulator answers one with a wake, which the expect would catch).
powerd screenSaver
power
wait 3000
expect asleep is 1
shot 02-missed-announcement-sleep-screen
# A press on the sleep screen, no outOfScreenSaver, and the power service still in its
# screensaver (a passcode waiting, say): Grimoire stays asleep, never drawing over the Kindle.
power
wait 3300
expect asleep is 1
shot 03-still-locked-stays-asleep
# Three side-button clicks while asleep start nothing (they cleared the page).
side on
side off
side on
side off
side on
side off
wait 400
expect asleep is 1
expect toast not Page cleared
# A press on the sleep screen, then the Kindle announces it is going to sleep (it was awake after
# all, Grimoire had missed the wake): that answers the press, so no wake 3 s later.
powerd active
power
sleep
wait 3300
expect asleep is 1
# The Kindle wakes: the ink is all there.
wake
wait 400
expect asleep is 0
expect screen is editor
shot 04-awake-ink-kept
# A wake announced while the key is still down: its release must not ask for sleep 2.5 s later.
sleep
wait 300
power down
wake
power up
wait 3000
expect asleep is 0
# A short press Grimoire took for a sleep press, then a wake announcement (it had missed the
# Kindle going to sleep): no sleep request after the wake either.
power
wake
wait 3000
expect asleep is 0
shot 05-no-sleep-after-wake
# Holding the power button 4 s saves the page and leaves Grimoire, the run ends here.
pen line 300 1000 1400 1020 40 0.6 0.6
powerhold
wait 300
# Never reached: if it were, this fails (the editor is open).
expect screen is library
shot 99-not-reached
