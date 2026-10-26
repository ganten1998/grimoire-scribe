# env: GRIMOIRE_UPDATE_SCRIPT=tests/fake_update.sh FAKE_UPDATE_FAIL=5
# Settings > Updates when the check cannot reach the feed (update.sh's status 5: no network): it says
# so, and Update stays off.
wait 150
tap 1535 95
wait 200
tap 1152 1930
wait 200
tap 930 1055
wait 600
expect library.update is failed network
shot 01-no-network
tap 930 1215
wait 100
expect running is 1
tap 930 1375
wait 100
expect library.sheet is none
