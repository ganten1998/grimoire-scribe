# settings: status_mode=1
# env: GRIMOIRE_UPDATE_SCRIPT=tests/fake_update.sh FAKE_UPDATE_PREVIEW=9.0.0-beta.9 FAKE_UPDATE_STABLE=1.2.0
# Settings > Updates, kindle/app/update.sh stood in for by tests/fake_update.sh: the version running;
# a check of the preview channel finds a future-version beta; releases only finds nothing newer
# (1.2.0), and Update stays off; betas again, a check, then Update now: Grimoire leaves (the request
# for the launcher: tests/test_update.c).
wait 150
tap 1535 95
wait 200
expect library.ui_at.1200.1930 is updates
tap 1200 1930
wait 200
expect library.sheet is updates
expect library.update is idle
shot 01-updates
tap 930 1055
wait 100
expect library.update is checking
wait 900
expect library.update is newer 9.0.0-beta.9
shot 02-newer
tap 930 895
wait 100
expect settings.updates is 1
expect library.update is idle
tap 930 1055
wait 1000
expect library.update is newest 1.2.0
shot 03-newest
tap 930 1215
wait 100
expect running is 1
expect update.requested is
tap 930 895
wait 100
expect settings.updates is 2
tap 930 1055
wait 1000
expect library.update is newer 9.0.0-beta.9
tap 930 1215
wait 300
expect running is 0
