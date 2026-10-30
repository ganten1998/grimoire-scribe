# rust-only
# generate: tests/notebook_links_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap library folder.Void
tap library notebook.A
tap panel links
pen line 1000 1100 1400 1300 20 0.6 0.6
pen leave
tap panel linkto.elsewhere
expect editor.destination.folder is Void
# Clear gestures must not act through the destination browser.
side on
wait 1300
side off
expect editor.links is 2 2
expect editor.strokes is 0
tap panel elsewhere.folder.Void/Nested
tap panel elsewhere.book.B
waitbg
tap panel elsewhere.back
expect editor.destination is 1
expect editor.destination.notebook is
expect editor.destination.folder is Void/Nested
tap panel elsewhere.back
expect editor.destination is 0
expect editor.linkto is pick
expect editor.links is 2 2
tap panel linkto.cancel
expect editor.link_mode is 0
# Picking a page in the origin notebook keeps a regular local page link.
tap panel links
pen line 1000 1100 1400 1300 20 0.6 0.6
pen leave
tap panel linkto.elsewhere
tap panel elsewhere.book.A
tap panel elsewhere.page.2
expect editor.notebook_id is A
expect editor.links is 3 3
tap panel undo
expect editor.links is 2 2
# Rotate inside the picker, then fail the actual sidecar save: no link or ink is left.
tap panel links
pen line 1000 1100 1400 1300 20 0.6 0.6
pen leave
tap panel linkto.elsewhere
tap panel elsewhere.folder.Void/Nested
tap panel elsewhere.book.B
waitbg
rotate 1
wait 300
expect editor.destination.notebook is B
shot 01-landscape-picker
storage full
tap panel elsewhere.page.2
expect editor.notebook_id is A
expect editor.links is 2 2
expect editor.strokes is 0
expect toast is Could not save the link.
storage ok
quit
