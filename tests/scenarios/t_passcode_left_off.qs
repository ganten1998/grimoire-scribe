# settings: status_mode=1
# env: GRIMOIRE_PASSCODE=left-off
# A session ended without putting the passcode back (the Kindle restarted under Grimoire): the
# launcher does not turn it back on itself, and tells Grimoire so (at this start and the next two,
# tests/launcher_check.sh checks). Grimoire notes it in grimoire.log only: no toast since 2.0.0-beta.2
# (the owner's toast pass), and the start is an ordinary one.
wait 300
expect screen is library
expect toast is
quit
