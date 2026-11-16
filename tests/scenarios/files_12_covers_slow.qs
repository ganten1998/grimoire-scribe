# settings: status_mode=1
# settings: files_card=1
# settings: files_view=covers
# storage: 1
# generate: tests/files_place_tree.py
# copy: tests/fixtures/pdf/planner documents/Covers
# copy: tests/fixtures/pdf/junk documents/Covers
# copy: tests/fixtures/photos documents/Covers
# copy: tests/fixtures/fonts/good documents/Covers
# env: GRIMOIRE_SIM_CLOCK=script
# env: GRIMOIRE_PIC_TEST_HANG=1
# env: GRIMOIRE_PREVIEW_MS=8000
# Files' Covers view (2.2, issue #145): a screen whose previews take long. The two photos, last of the eight, hang
# their workers here, and each is ended when the preview's own time is up (8 s here; 6 s for a picture on the
# Kindle, where an import would wait 60 s and try a second worker). While the screen's set is not complete, the
# tiles that changed are drawn together once every 2 s at most: the six files before the photos are made within
# the first seconds and nothing is refreshed while the app's clock stands still (waitbg); 2 s on, the six go out
# as one ui refresh (after the notice's own, at its first second); the photos' tiles, when the second one's time is
# up and the set is complete, go out together as one more (the notice's place, which that refresh reaches, cleared
# with it). Two refreshes of tiles for eight files, none a full.
# A file that ran out of time once keeps its kind's icon for the visit, and has no mark yet.
# rust-only
clock 1700000000
wait 3600
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 400
expect display.presents not -1
waitbg 4000
expect files.tiles is llllllll
expect display.presents is
wait 1900
expect notice is Making previews...
expect files.tiles is pippppll
expect display.presents is ui 677 2250 505 110, ui 46 356 1766 1284
shot 01-six-made
waitbg
wait 60
expect files.tiles is pippppii
expect files.making is 0
expect notice is
expect display.presents is ui 46 1662 1170 698
shot 02-photos-out-of-time
quit
