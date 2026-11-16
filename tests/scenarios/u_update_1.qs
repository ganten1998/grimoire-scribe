# settings: status_mode=1
# settings: updates=preview
# (The preview channel pinned: on Automatic, a stable version such as 2.0.0 follows releases only.)
# env: GRIMOIRE_UPDATE_SCRIPT=tests/fake_update.sh FAKE_UPDATE_PREVIEW=9.0.0-beta.9 FAKE_UPDATE_STABLE=1.2.0
# Settings > Updates, kindle/app/update.sh stood in for by tests/fake_update.sh: the page checks as it
# opens (the new Settings menu, #115), and the preview channel offers a future-version beta; releases only
# (the switch checks again) finds nothing newer (1.2.0), and Update stays off; betas again, then Update:
# Grimoire leaves (the request for the launcher: tests/test_update.c).
wait 150
tap 1535 95
wait 200
tap library settings.cat.0
wait 100
expect library.ui_at.1233.505 is updates
tap 1233 505
wait 100
expect library.sheet is updates
expect library.update is checking
shot 01-updates
wait 900
expect library.update is newer 9.0.0-beta.9
shot 02-newer
tap library updates.channel
wait 100
expect settings.updates is 1
expect library.update is checking
wait 1000
expect library.update is newest 1.2.0
shot 03-newest
expect library.ui_at.930.301 is
tap 930 301
wait 100
expect running is 1
expect update.requested is
tap library updates.channel
wait 100
expect settings.updates is 2
wait 1000
expect library.update is newer 9.0.0-beta.9
tap library updates.now
wait 300
expect running is 0
