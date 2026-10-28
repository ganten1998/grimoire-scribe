# Compaction runs in the background once the notebook closes, and opening the notebook stops it
# first: then its page file is as it was, and the next close compacts it. GRIMOIRE_BG_DELAY_MS makes
# each background job wait 3 s first, as a compaction of a heavily erased page can take that long
# on the Kindle. editor.matches_closed: the page shows as its file did when the notebook closed.
# env: GRIMOIRE_BG_DELAY_MS=3000
# settings: eraser_mode=1
# settings: mirror_pdf=0
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
pen line 300 700 1300 720 60 0.6 0.6
pen line 300 1000 1300 1000 60 0.6 0.6
# The first line erased entirely by one pass, the second in part
eraser line 250 710 1350 710 60
eraser line 800 940 840 1060 10
wait 300
expect editor.strokes is 4
expect editor.erase_strokes is 2
expect editor.live_matches_render is 1
pentap panel library
wait 300
expect screen is library
expect bg.busy is 1
# Opened while the compaction waits: it is stopped, and the page is whole
tap 326 608
wait 500
expect screen is editor
expect bg.busy is 0
expect editor.strokes is 4
expect editor.erase_strokes is 2
expect editor.matches_closed is 1
expect editor.live_matches_render is 1
shot 01-reopened-during-compaction
# Closed again: this time the compaction gets through
pentap panel library
wait 300
expect bg.busy is 1
wait 4500
expect bg.busy is 0
tap 326 608
wait 500
expect screen is editor
expect editor.strokes is 2
expect editor.erase_strokes is 1
expect editor.matches_closed is 1
expect editor.live_matches_render is 1
shot 02-compacted
pentap panel library
wait 300
expect bg.busy is 0
quit
