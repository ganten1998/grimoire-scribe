# settings: status_mode=1
# A page with erased ink is listed for compaction (erased.txt) once its file has the erase stroke,
# at a save, and a save that fails lists nothing: with the store full (the simulator's "storage
# full") the autosave fails and the page is not listed; once there is room the next autosave writes
# the page and lists it, and closing the notebook compacts it (the erased line and the pass over it
# leave the file).
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
wait 2600
expect editor.unsaved is 0
expect editor.compact_listed is 0
storage full
eraser line 250 710 1350 710 60
wait 2600
expect editor.unsaved is 1
expect editor.erase_strokes is 1
expect editor.compact_listed is 0
storage ok
wait 2600
expect editor.unsaved is 0
expect editor.compact_listed is 1
pentap panel library
wait 1500
expect screen is library
expect bg.busy is 0
tap 326 608
wait 500
expect screen is editor
expect editor.strokes is 1
expect editor.erase_strokes is 0
expect editor.compact_listed is 0
expect editor.live_matches_render is 1
pentap panel library
wait 300
expect screen is library
quit
