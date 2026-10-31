# settings: status_size=0
# rust-only
# generate: tests/page_management_fixture.py
# settings: mirror_pdf=0
# env: GRIMOIRE_CHECK_FIT=1 TZ=UTC
wait 300
clock 1700000000
battery 87
tap library folder.Void
tap library notebook.A
expect settings.status_on is 1
expect settings.status_size is 0
# Toolbar count opens management; a single selection replaces the last until Multi-select.
pentap panel pages
waitbg
expect editor.modal is pages
expect editor.pages.grid is 3 3
expect editor.pages.multi is 0
expect editor.pages.cached is 9
tap panel pages.page.2
tap panel pages.page.3
expect editor.ticked is 1
tap panel pages.multi
tap panel pages.page.4
expect editor.pages.multi is 1
expect editor.ticked is 2
expect editor.page is 1
shot 01-multi-borders-and-icons
rotate 1
expect editor.ticked is 2
shot 02-landscape-multi
rotate 0
tap panel pages.multi
expect editor.ticked is 0
tap panel pages.back
# Name stays over the notebook and closes back to that page.
pen leave
wait 1100
swipe 2 930 1400 0 -400
tap panel bar.name
expect editor.pages.overlay is 1
expect editor.pages.keyboard is Page name
expect editor.screen_composed is 1
shot 03-name-over-page
tap 133 1725
tap 1643 2365
expect editor.modal is none
expect editor.page is 1
expect editor.screen_composed is 1
# The Links bar's Go to page opens the Find sheet over the page, as the go-to-page action does (a
# number or a name), never the Pages view.
pen leave
wait 1100
swipe 2 930 1400 0 -400
tap panel bar.goto
expect editor.find is go
expect editor.modal is find
expect editor.bar is 0
expect editor.keyboard is Page number or name
expect editor.screen_composed is 1
shot 04-find-over-page
tap 487 1725
expect editor.find.1 is Go to page 3
tap 930 1390
wait 300
expect editor.page is 3
expect editor.modal is none
expect editor.screen_composed is 1
# Changing size/corner with the pen still near paints the actual clock before menu exit.
pentap panel display
expect editor.modal is display
wait 100
expect status.shown is 1
pentap panel display.size
wait 100
expect settings.status_size is 1
expect status.shown is 1
expect status.rect not 6 6 331 48
shot 05-clock-live-preview
pentap panel display.corner
wait 100
expect settings.status_corner is 1
expect status.shown is 1
pentap panel display.done
expect editor.modal is none
pentap panel shapes
expect editor.shapes_menu is 1
shot 06-shapes-theme-heading
pentap panel shapes.done
# Swipe down still opens the default Graph.
pen leave
wait 1100
swipe 2 930 900 0 400
expect editor.modal is map
quit
