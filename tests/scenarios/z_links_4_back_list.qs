# rust-only
# generate: tests/notebook_backlinks_fixture.py back
# env: GRIMOIRE_CHECK_FIT=1
# Back across notebooks brings back that notebook's own back list (2.0.0-beta.2): A's page 1 links to its
# page 2, whose link goes to B's page 1. Back from B returns to A's page 2 with page 1 still to go back to.
wait 300
tap library folder.Void
tap library notebook.A
expect editor.notebook_id is A
pen leave
wait 1100
tap 500 600
expect editor.page is 2
expect editor.back is 1 1
pen leave
wait 1100
tap 500 600
expect editor.notebook_id is B
expect editor.page is 1
expect editor.notebook_back is 1 A
expect editor.back is 0 0
key 48 0
expect editor.notebook_id is A
expect editor.page is 2
expect editor.notebook_back is 0
expect editor.back is 1 1
shot 01-back-with-its-list
key 48 0
expect editor.notebook_id is A
expect editor.page is 1
expect editor.back is 0 0
quit
