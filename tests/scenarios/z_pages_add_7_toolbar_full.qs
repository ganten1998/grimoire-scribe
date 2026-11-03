# rust-only
# generate: tests/add_pages_fixture.py
# settings: shortcuts_enabled=1
# copy: tests/fixtures/new-pages-key .
# The toolbar's + at the limit (owner, 2026-11-03): held on the last page of a notebook with room for five, the
# pad takes at most five; once the notebook is full, the + held on its last page says so and opens no pad (its
# press let go), and a plain tap on it says it could not add a page, as before. Opening one of the batch's pages
# (going to it) makes it a page in use: the Pages view no longer offers the batch's Undo. (No fit check here: the
# panel's page count "1995 / 2000" does not fit its chip, docs/BACKLOG.md.)
wait 300
tap library folder.Void
tap library notebook.F
key 37 0
wait 300
expect editor.page is 1995
touch down 1768 1330
wait 700
touch up 1768 1330
wait 100
expect editor.addpad is 1
key 10 0
wait 100
expect editor.text is 5
expect toast is This notebook has room for 5 more pages.
key 28 0
wait 300
expect editor.addpad is 0
expect editor.pages is 2000
expect editor.page is 1995
expect toast is Added 5 pages after this one, from page 1996. Undo them in Pages.
key 37 0
wait 300
expect editor.page is 2000
touch down 1768 1330
wait 700
expect editor.addpad is 0
expect toast is This notebook is full: it has 2000 pages.
touch up 1768 1330
wait 200
expect editor.addpad is 0
expect editor.page is 2000
expect editor.pages is 2000
tap panel next
wait 300
expect editor.pages is 2000
expect editor.page is 2000
expect toast is Could not add a page
tap panel pages
waitbg
wait 200
expect editor.pages.undo is 0
quit
