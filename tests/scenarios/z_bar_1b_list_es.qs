# rust-only
# The Links bar in Spanish, upright and sideways, under the fit check: nothing is cut that should not be.
# root: z_bar_0_make
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
wait 300
tap 200 400
wait 300
expect screen is editor
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 2
expect editor.bar.1 is p3 x2
expect editor.bar.2 is p2
expect editor.bar.rect is 20 1944 1820 516
shot 01-bar-upright-es
key 37 0
wait 300
expect editor.bar is 0
rotate 1
wait 1200
swipe 2 1240 1300 0 -400
wait 300
expect editor.bar is open
shot 02-bar-sideways-es
