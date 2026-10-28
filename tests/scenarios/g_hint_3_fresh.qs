# rust-only
# A fresh install has no settings file to have upgraded: the first notebook opens without the tip, and
# the first turn sideways without the sideways hint.
# settings: mirror_pdf=0
wait 300
expect settings.hints is 0
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect toast is
expect settings.hints is 0
rotate 1
wait 1200
expect toast is
expect settings.hints is 0
