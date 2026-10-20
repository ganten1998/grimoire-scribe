# env: GRIMOIRE_PASSCODE=left-off
# A session ended without putting the passcode back (the Kindle restarted under Grimoire): the
# launcher does not turn it back on itself, Grimoire tells the user to (at this start and the
# next two, tests/launcher_check.sh checks), before any other notice.
wait 300
expect toast is Your Kindle passcode was left off. Turn it back on in Kindle Settings.
quit
