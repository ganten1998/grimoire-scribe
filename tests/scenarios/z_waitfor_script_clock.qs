# env: GRIMOIRE_SIM_CLOCK=script
# copy: tests/fixtures/update-failed
# rust-only
# waitfor under the script clock (docs/SIM.md): while it waits, app time moves on by what it idled, as in a
# wait, so it can wait on what a timer does. Here the 6 s notice a start shows after an update that did not
# install (the fixture's .update-failed): still up 5.3 s of app time in, gone once its 6 s are over.
wait 300
expect toast is Last update failed. See grimoire.log.
wait 5000
expect toast is Last update failed. See grimoire.log.
waitfor toast is
expect toast is
expect screen is library
quit
