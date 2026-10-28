# settings: status_mode=1
# Open Share (tests/share_check.sh holds a copy open while its client downloads, then stops it;
# make scenarios runs this script as it is)
wait 300
expect screen is library
tap 1345 95
wait 600
shot 01-share
expect screen is share
shot 02-share-after
expect screen is share
quit
