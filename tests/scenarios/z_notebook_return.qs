# rust-only
# generate: tests/notebook_links_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap library folder.Void
tap library notebook.A
pen leave
wait 1100
tap 500 1000
expect editor.notebook_id is C
expect editor.page is 2
# The page-status Back chip returns across notebooks.
swipe 2 930 1300 0 -400
wait 300
tap panel bar.back
expect editor.notebook_id is A
expect editor.page is 1
pen leave
wait 1100
tap 500 1000
expect editor.notebook_id is C
pen line 1000 1500 1400 1600 20 0.6 0.6
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
storage full
tap panel bar.back
expect editor.notebook_id is C
expect editor.strokes is 1
storage ok
tap panel bar.back
expect editor.notebook_id is A
pen leave
wait 1100
tap 500 1000
expect editor.notebook_id is C
expect editor.page is 2
expect editor.strokes is 1
# A manual open remembers the linked destination, even without subsequent editing.
tap panel library
waitbg
tap 125 95
expect library.folder is
tap library folder.Other
tap library notebook.C
expect editor.notebook_id is C
expect editor.page is 2
expect editor.strokes is 1
quit
