# settings: status_mode=1
# env: GRIMOIRE_UPDATE_SCRIPT=tests/fake_update.sh FAKE_UPDATE_FAIL=5
# Settings > Updates when the check (as the page opens) cannot reach the feed (update.sh's status 5: no
# network): it says so, and its button asks again rather than updating; X closes Settings.
wait 150
tap library settings
wait 200
tap library settings.cat.0
wait 100
expect library.ui_at.1233.505 is updates
tap 1233 505
wait 200
wait 600
expect library.update is failed network
shot 01-no-network
tap library updates.now
wait 100
expect running is 1
tap library settings.close
wait 100
expect library.sheet is none
