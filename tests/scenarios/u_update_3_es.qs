# settings: status_mode=1
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1 GRIMOIRE_UPDATE_SCRIPT=tests/fake_update.sh FAKE_UPDATE_PREVIEW=2.0.0-beta.9 FAKE_UPDATE_STABLE=1.2.0
# Settings > Updates in Spanish (the longer language), upright and sideways, every text checked for
# fit: the version, a newer one with its Update row, releases only and the newest.
wait 150
tap 1535 95
wait 200
tap 1152 1930
wait 200
shot 01-updates
tap 930 1055
wait 1000
expect library.update is newer 2.0.0-beta.9
shot 02-newer
tap 930 895
wait 100
tap 930 1055
wait 1000
expect library.update is newest 1.2.0
shot 03-newest
rotate 1
wait 300
shot 04-newest-landscape
