# Open Share and stay there while a client downloads (driven by tests/share_check.sh)
wait 300
expect screen is library
tap 1345 95
wait 600
shot 01-share
expect screen is share
wait 25000
shot 02-share-after
expect screen is share
quit
