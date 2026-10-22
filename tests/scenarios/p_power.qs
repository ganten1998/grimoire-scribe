# The power button's ways out of every stuck state: a sleep announcement Grimoire missed, a Kindle
# still locked after a press, the side button and pinches while asleep, a wake announced while the
# key is down, a press held short of 4 s, a press that follows another, and the 4 s hold that saves
# and leaves Grimoire.
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
wait 3300
expect asleep is 1
shot 02-missed-announcement-sleep-screen
# A press on the sleep screen, no outOfScreenSaver, and the power service still in its
# screensaver (a passcode waiting, say): Grimoire stays asleep, never drawing over the Kindle.
power
wait 3300
expect asleep is 1
shot 03-still-locked-stays-asleep
# Three side-button clicks and a pinch while asleep start nothing (the clicks cleared the page,
# the pinch zoomed it behind the sleep screen).
side on
side off
side on
side off
side on
side off
pinch 930 1240 300 900 8
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
# The Kindle wakes: the ink is all there, at 100% (no zoom chip).
wake
wait 400
expect asleep is 0
expect screen is editor
shot 04-awake-ink-kept
expect editor.zoom is 1.00
expect editor.strokes is 2
# Grimoire thinks it is awake (it missed the Kindle going to sleep), the button goes down, and the
# wake is announced while it is still down: its release must not ask for sleep 2.5 s later.
power down
wake
power up
wait 3000
expect asleep is 0
# The same on the sleep screen: a release is judged by the screen its press went down on.
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
# The button held 2.6 s: past the 2.5 s a short press waits for its announcement, and far enough
# short of the 4 s hold that a busy machine, which can only stretch a wait, does not reach it.
# Nothing while it is down (no sleep request halfway through a hold)...
power down
wait 2600
expect asleep is 0
# ...and its release is a short press: no announcement follows, so Grimoire asks for sleep.
power up
shot 06-released-short-of-4s-still-awake
wait 3000
expect asleep is 1
wake
wait 400
expect asleep is 0
# A short press, then the button held (press, see nothing, then hold: what a stuck user does):
# the first press's sleep request must not go out while the second is down (held past the 2.5 s
# that request waits, and 2 s short of the hold).
power
wait 1000
power down
wait 2000
expect asleep is 0
shot 07-second-press-held-still-awake
power up
sleep
wait 300
wake
wait 400
expect asleep is 0
# Holding the power button 4 s saves the page and leaves Grimoire at the 4 s mark, with the key
# still down: the run ends here. (A release held 4 s whose hold never came: p_power_released_late.)
pen line 300 1000 1400 1020 40 0.6 0.6
power down
wait 100
power held
wait 300
expect running is 0
