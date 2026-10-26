# settings: shortcuts_enabled=1
# settings: controller=1
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
# The Pair sheet's every state in Spanish (the longer language) upright and sideways, its texts
# checked for fit: searching, the controllers found, pairing, a controller paired (its Forget row).
wait 2000
tap 1535 95
wait 200
tap 945 1930
wait 200
tap 930 895
wait 300
shot 01-searching
wait 3000
shot 02-found
tap 930 895
wait 300
shot 03-pairing
wait 2500
shot 04-paired
rotate 1
wait 300
shot 05-paired-landscape
quit
