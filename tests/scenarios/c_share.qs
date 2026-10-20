# Open Share (tests/share_check.sh runs a copy that stays 25 s longer before shot 02, while its
# client downloads; make scenarios runs it as it is)
wait 300
expect screen is library
tap 1345 95
wait 600
shot 01-share
expect screen is share
shot 02-share-after
expect screen is share
quit
