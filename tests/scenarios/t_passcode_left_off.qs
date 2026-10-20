# env: GRIMOIRE_PASSCODE=left-off
# A session ended without putting the passcode back (the Kindle restarted under Grimoire): the
# launcher does not turn it back on itself, Grimoire tells the user to.
wait 300
expect toast has passcode was left off
quit
