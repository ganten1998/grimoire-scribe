# rust-only
# generate: tests/notebook_links_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap library folder.Void
tap library notebook.A
expect editor.links is 2 2
tap panel links
pen line 1000 1100 1400 1300 20 0.6 0.6
pen leave
expect editor.linkto is pick
tap panel linkto.elsewhere
expect editor.destination is 1
expect editor.destination.folder is Void
shot 01-directory
tap panel elsewhere.up
expect editor.destination.folder is
tap panel elsewhere.folder.Void
tap panel elsewhere.folder.Void/Nested
expect editor.destination.folder is Void/Nested
tap panel elsewhere.book.B
expect editor.destination.notebook is B
waitbg
wait 300
shot 02-destination-pages
tap panel elsewhere.page.2
expect editor.destination is 0
expect editor.notebook_id is A
expect editor.links is 3 3
expect editor.strokes is 0
tap panel undo
expect editor.links is 2 2
tap panel redo
expect editor.links is 3 3
tap panel pages
expect editor.map.foreign is 2
shot 03-notebook-groups
tap panel map.list
shot 04-grouped-links
tap panel map.back
pen leave
wait 1100
tap 1200 1200
expect editor.notebook_id is B
expect editor.page is 2
key 48 0
expect editor.notebook_id is A
expect editor.page is 1
tap panel library
waitbg
tap library notebook.A
expect editor.links is 3 3
pen leave
wait 1100
tap 1200 1200
expect editor.notebook_id is B
expect editor.page is 2
# Delete only the destination page; its old link must fail safely without changing the origin.
tap panel pages
tap panel map.pages
waitbg
tap 828 296
tap panel pages.delete
tap panel pages.back
key 48 0
expect editor.notebook_id is A
expect editor.pages is 2
pen leave
wait 1100
tap 1200 1200
expect editor.notebook_id is A
expect editor.links is 3 3
expect toast not
tap panel pages
expect editor.map.foreign is 2
shot 05-deleted-destination
tap panel map.back
quit
