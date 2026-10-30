# settings: status_mode=1
# rust-only
# Back to the page before: a link, the Pages view and a Find result leave the page they came from on the
# back list (at most 32, memory only), a page turn does not, and the action pops the newest, with a toast
# when there is none.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/index-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
wait 100
pentap panel next
wait 100
pentap panel previous
wait 100
pentap panel previous
wait 100
expect editor.pages is 3
expect editor.page is 1
expect editor.back is 0 0
# nothing to go back to
key 48 0
wait 200
expect toast is Nothing to go back to.
expect editor.page is 1
# a page turn leaves nothing behind
pentap panel next
wait 300
expect editor.page is 2
expect editor.back is 0 0
pentap panel previous
wait 300
# a link's tap does
key 38 0
pen line 300 1000 900 1200 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
expect editor.picking is new
tap 1155 340
expect editor.link.1 is p3
tap 600 1100
wait 300
expect editor.page is 3
expect editor.back is 1 1
key 48 0
wait 300
expect editor.page is 1
expect editor.back is 0 0
# the Pages view's Open does too
pentap panel pages
wait 300
tap panel pages.page.2
tap panel pages.page.2
wait 400
expect editor.page is 2
expect editor.back is 1 1
key 48 0
wait 300
expect editor.page is 1
expect editor.back is 0 0
# the same page again leaves nothing, and a stack goes back one page at a time
tap 600 1100
wait 300
expect editor.page is 3
expect editor.back is 1 1
pentap panel pages
wait 300
tap panel pages.page.2
tap panel pages.page.2
wait 400
expect editor.page is 2
expect editor.back is 2 3
key 48 0
wait 300
expect editor.page is 3
expect editor.back is 1 1
key 48 0
wait 300
expect editor.page is 1
expect editor.back is 0 0
# link-areas flips the eye's setting (what it draws is the eye's own scenario)
key 30 0
expect settings.link_areas is 1
key 30 0
expect settings.link_areas is 0
