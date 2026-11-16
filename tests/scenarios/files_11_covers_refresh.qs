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
# env: GRIMOIRE_BG_DELAY_MS=150
# Files' Covers view (2.2, issue #145): what a screen of previews costs the panel. Previews that arrive are never
# one refresh a file: the tiles that changed are drawn together, at once when the screen's set is complete. Here
# eight files are made one after another (each job held back 150 ms) while the app's clock stands still inside the
# 2 s between tile refreshes (the script's clock: waitbg lets the jobs run and moves no app time): no tile is
# refreshed until the last preview is in, and then all eight go out as ONE ui refresh of their union, none of them
# a full. Entered again with every preview in the cache, nothing is made or asked for again: the tiles are read
# from the cache, and go out with the folder's own refresh or as one more. (The clock corner held still: its
# minute's refresh would land among the counted ones.)
# rust-only
clock 1700000000
wait 3600
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 400
expect files.tiles is llllllll
expect display.presents not -1
waitbg
wait 60
expect files.tiles is pipppppp
expect files.making is 0
expect display.presents is ui 46 356 1766 1937
shot 01-made
tap library files.back
wait 300
tap library files.item.Covers
waitfor files.tiles is pipppppp
expect files.asked is 8
expect bg.busy is 0
quit
