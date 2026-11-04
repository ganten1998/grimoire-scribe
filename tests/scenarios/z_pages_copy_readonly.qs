# rust-only
# generate: tests/lasso_more_fixture.py
# Copy page of a read-only page (lasso-big's page 3 holds a chunk this version does not know): the copy opens,
# read-only as its source is, and says so as any read-only page does when it opens. Since 2.0.0-beta.2 no toast
# says the copy was made (the owner's toast pass), and taking down a toast left up from before must not take
# that notice with it.
wait 300
tap 322 620
wait 1000
expect screen is editor
expect editor.page is 1
expect editor.strokes is 2
pentap panel pages
wait 300
tap panel pages.page.3
wait 100
expect editor.ticked is 1
tap panel pages.copy
wait 300
expect editor.modal is none
expect editor.pages is 4
expect editor.page is 4
expect editor.strokes is 1
expect toast is This page was made by a newer version, and will be viewed as read-only.
shot 01-copy-read-only
quit
