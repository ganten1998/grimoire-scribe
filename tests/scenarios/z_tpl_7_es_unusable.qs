# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# A templates folder of files that are no pictures Grimoire can use: the Templates sheet offers
# none, says how to add some and how many files it cannot use, in Spanish with the fit check, in
# Gothic and Classic, upright and sideways.
# copy: tests/fixtures/tpl-bad templates
wait 300
tap 965 95
wait 100
tap 300 1650
wait 200
expect library.sheet is templates
# (the previews job finds them unusable, and the sheet takes them out: up to 7 s on a busy machine)
wait 7000
expect templates.count is 0
shot 01-unusable-gothic
rotate 1
wait 300
expect library.sheet is templates
shot 02-unusable-gothic-sideways
tap 300 1694
wait 100
tap 570 1700
wait 100
rotate 0
wait 300
tap 1535 95
wait 100
tap 930 930
wait 100
expect settings.theme is classic
tap 1560 1930
wait 100
tap 965 95
wait 100
tap 300 1650
wait 200
expect templates.count is 0
shot 03-unusable-classic
rotate 1
wait 300
shot 04-unusable-classic-sideways
tap 300 1694
wait 100
tap 570 1700
wait 100
rotate 0
wait 300
quit
