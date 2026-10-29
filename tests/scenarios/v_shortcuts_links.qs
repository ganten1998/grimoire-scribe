# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/link-keys .
# Add a link (L, a controller's West) starts a link on the open page in one press, as the Index's
# Add a link does but without the Pages view: the strip comes up, the next pen stroke is the dashed
# lasso around its area (no ink), then the Pages view picks where it goes (a page, the Index). Pressed
# again while the strip is up, it is not made. Name the page (N, a controller's North) opens the
# Index's naming keyboard for the open page, holding its name; its Done saves the name and its Cancel
# keeps the old one, and either goes back to the page; one that cannot be saved says so, back on the
# page. Neither acts through the keyboard, and the Pages view opens again on the tab it was left on,
# not the Index shown under the keys.
# rust-only
wait 150
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
pen line 300 600 900 620 30 0.5 0.9
expect editor.strokes is 1
# Add a link, and again: not made
key 38 0
wait 60
expect editor.link_mode is 1
expect editor.modal is none
expect editor.strip is new
shot 01-add-a-link
key 38 0
wait 60
expect editor.link_mode is 0
expect toast is No link made.
expect editor.strokes is 1
# One press, the box, then its page
key 38 0
wait 60
expect editor.link_mode is 1
pen line 300 1000 900 1200 30 0.6 0.6
expect editor.strokes is 1
expect editor.link_mode is 0
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
expect editor.modal is pages
expect editor.picking is new
expect toast is Where does the link go? Tap its page.
shot 02-where-it-goes
tap 1155 340
expect editor.modal is none
expect editor.page is 1
expect editor.links is 1 1
expect editor.link.1 is p3
expect toast is Linked to page 3.
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# The controller's button, to the Index
key 308 0 1
wait 60
expect editor.link_mode is 1
pen line 300 1600 900 1800 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.679 is linkto.pages
tap 930 679
expect editor.picking is new
tap 1440 95
expect editor.modal is none
expect editor.links is 2 2
expect toast is Linked to the Index.
# (links.ini orders a page's links by their seeds: a finger's tap tells which goes where)
tap 600 1700
expect editor.modal is pages
expect editor.pages_tab is index
expect editor.index_rows is 0
tap 125 95
expect editor.modal is none
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
wait 2600
# Name the page: its keyboard over the Index, Done back to the page
expect editor.page_name is
key 49 0
wait 60
expect editor.modal is pages
expect editor.pages_tab is index
expect editor.keyboard is Page name
expect editor.text is
shot 03-name-the-page
# (nothing acts through the keyboard)
key 38 0
wait 60
expect editor.link_mode is 0
expect editor.keyboard is Page name
tap 175 2205
tap 1455 2205
tap 221 2045
tap 1726 1885
expect editor.text is Map
tap 1643 2365
expect editor.modal is none
expect editor.keyboard is
expect editor.page_name is Map
expect editor.index_rows is 1
expect editor.page is 1
shot 04-named
# The link to the Index lists it
tap 600 1700
expect editor.modal is pages
expect editor.pages_tab is index
expect editor.index_rows is 1
expect editor.keyboard is
tap 125 95
expect editor.modal is none
# Again from the controller, holding the name; Cancel keeps it
key 307 0 1
wait 60
expect editor.keyboard is Page name
expect editor.text is Map
tap 1679 2205
expect editor.text is Ma
tap 217 2365
expect editor.modal is none
expect editor.page_name is Map
# A name that cannot be saved: the old one stays, and the toast says so on the page
storage full
key 49 0
wait 60
tap 1679 2205
tap 1643 2365
expect editor.modal is none
expect editor.page_name is Map
expect toast is Could not save the page name.
storage ok
# On another page, its own name
pentap panel next
expect editor.page is 2
key 49 0
wait 60
expect editor.text is
tap 175 2205
tap 1278 2205
tap 1549 1885
tap 841 1885
tap 487 1885
tap 398 2045
tap 1643 2365
expect editor.page_name is Notes
expect editor.index_rows is 2
# Left on its Pages tab, the Pages view opens there again after naming, by Cancel or by Done
pentap panel pages
expect editor.modal is pages
tap 430 95
expect editor.pages_tab is pages
tap 125 95
expect editor.modal is none
key 49 0
wait 60
expect editor.pages_tab is index
expect editor.keyboard is Page name
tap 217 2365
expect editor.modal is none
pentap panel pages
expect editor.modal is pages
expect editor.pages_tab is pages
tap 125 95
key 49 0
wait 60
expect editor.text is Notes
tap 1643 2365
expect editor.modal is none
expect editor.page_name is Notes
pentap panel pages
expect editor.pages_tab is pages
tap 125 95
expect editor.modal is none
quit
