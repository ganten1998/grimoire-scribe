# env: GRIMOIRE_PASSCODE=paused
# The launcher paused the Kindle's Device Passcode for the session (kindle/app/grimoire.sh): the
# first time, Grimoire says so.
wait 300
expect screen is library
expect toast has passcode is paused while Grimoire is open
quit
