# Grimoire leaves with a notebook open (quit): its page with erased ink stays listed (erased.txt)
# instead of a compaction Grimoire's leaving would cut short, and the next start compacts it
# (z_9_compact_at_start).
# settings: eraser_mode=1
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
pen line 300 700 1300 720 60 0.6 0.6
pen line 300 1000 1300 1000 60 0.6 0.6
# The first line erased entirely by one pass
eraser line 250 710 1350 710 60
wait 2600
expect editor.unsaved is 0
expect editor.strokes is 3
expect editor.erase_strokes is 1
quit
