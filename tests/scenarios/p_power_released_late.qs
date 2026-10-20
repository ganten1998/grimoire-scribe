# The power button held 4 s while Grimoire was too busy to see the 4 s mark: its release, held 4 s
# or more, leaves Grimoire just as the hold would have (the runner never sends the hold here).
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen line 300 760 1400 780 60 0.4 0.9
power down
wait 4100
expect running is 1
power up
wait 300
expect running is 0
