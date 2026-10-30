# rust-only
# generate: tests/notebook_links_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap library folder.Void
tap library notebook.A
expect screen is editor
expect editor.notebook_id is A
tap panel pages
expect editor.map is graph
expect editor.map.list is 0
expect editor.map.foreign is 1
shot 01-graph
tap panel map.list
expect editor.map.list is 1
shot 02-list
tap panel map.pages
expect editor.modal is pages
waitbg
wait 300
shot 03-pages-thumbnails
tap panel pages.graph
expect editor.map is graph
tap panel map.back
expect editor.modal is none
tap panel settings
expect screen is library
expect library.sheet is settings
tap library done
expect screen is editor
expect editor.notebook_id is A
expect editor.page is 1
pen leave
wait 1100
swipe 2 930 1400 0 -400
expect editor.bar is open
shot 04-page-status
tap panel bar.name
expect editor.modal is pages
expect editor.keyboard not
key 1 0
quit
