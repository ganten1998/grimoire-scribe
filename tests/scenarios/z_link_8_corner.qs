# settings: status_mode=1
# settings: status_on=0
# settings: status_corner=2
# rust-only
# root: z_bar_0_make
# The clock corner at the bottom right (zone 1695 2315 165 165) and link making (PHASE2 rule 12): the strip and
# Link to... docked at the bottom end 8 px above the corner's zone, so neither meets it; a tap in the corner toggles
# the clock and cancels nothing; the strip's Cancel, above the corner, still cancels.
wait 300
tap 200 400
wait 300
expect editor.page is 1
expect status.zone is 1695 2315 165 165
key 38 0
expect editor.strip is new
expect editor.ui_at.1700.2400 is
expect editor.ui_at.1700.2240 is strip.cancel
tap 1780 2400
wait 100
expect settings.status_on is 1
expect editor.strip is new
tap 1780 2400
wait 100
expect settings.status_on is 0
expect editor.strip is new
tap 1700 2240
wait 100
expect editor.strip is 0
expect editor.link_mode is 0
expect settings.status_on is 0
# Link to... at the bottom (the area is high on the page), above the corner
key 38 0
pen line 1000 600 1500 800 30 0.6 0.6
pen leave
wait 1100
expect editor.linkto is pick
expect editor.linkto.rect is 20 1711 1820 596
tap 1780 2400
wait 100
expect settings.status_on is 1
expect editor.linkto is pick
tap 1780 2400
wait 100
expect settings.status_on is 0
expect editor.linkto is pick
