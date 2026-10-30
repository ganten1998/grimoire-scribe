# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/link-keys .
# Keyboard/controller link actions arm a lasso, cancel cleanly and pick local pages.
# The notebook destination browser replaces The Index destination. Naming keeps its Done,
# Cancel, save-failure and keyboard-blocking behavior; the toolbar always reopens Graph.
# copy: tests/fixtures/scen-tpl-nobg/notebooks/20261020-000000-0005 notebooks/link-origin
# settings: mirror_pdf=0
# rust-only
wait 150
tap library notebook.link-origin
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
tap panel linkto.pages
expect editor.modal is pages
expect editor.picking is new
expect toast is Where does the link go? Tap its page.
shot 02-where-it-goes
tap panel pages.page.3
expect editor.modal is none
expect editor.page is 1
expect editor.links is 1 1
expect editor.link.1 is p3
# (the first link this data root makes: the one-time tip instead of where it goes, hints bit 1)
expect toast is Linked. Tap it with a finger to go there; hold it for more.
expect settings.hints is 2
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# The controller's West button also offers the destination browser.
key 308 0 1
wait 60
expect editor.link_mode is 1
pen line 300 1600 900 1800 30 0.6 0.6
expect editor.strokes is 1
expect editor.linkto is pick
tap panel linkto.elsewhere
expect editor.destination is 1
tap panel elsewhere.book.link-origin
expect editor.destination.notebook is link-origin
tap panel elsewhere.page.2
expect editor.destination is 0
expect editor.modal is none
expect editor.links is 2 2
expect toast is Linked to page 2.
# Both boxes navigate to their own local pages, without opening a browser.
tap 600 1700
expect editor.page is 2
expect editor.modal is none
pentap panel previous
expect editor.page is 1
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
wait 2600
# Name the page: Done returns to the notebook
expect editor.page_name is
key 49 0
wait 60
expect editor.modal is pages
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
# The named page appears in Graph and in page management.
pentap panel pages
expect editor.map is graph
expect editor.map.list is 0
expect editor.index_rows is 1
tap panel map.pages
expect editor.modal is pages
expect editor.tile.1 has Map
expect editor.keyboard is
tap panel pages.back
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
# Management no longer changes the default Graph entry after naming Cancel or Done.
pentap panel pages
expect editor.map is graph
tap panel map.pages
expect editor.modal is pages
expect editor.pages_tab is pages
tap panel pages.back
expect editor.modal is none
key 49 0
wait 60
expect editor.keyboard is Page name
tap 217 2365
expect editor.modal is none
pentap panel pages
expect editor.map is graph
expect editor.map.list is 0
tap panel map.back
key 49 0
wait 60
expect editor.text is Notes
tap 1643 2365
expect editor.modal is none
expect editor.page_name is Notes
pentap panel pages
expect editor.map is graph
expect editor.map.list is 0
tap panel map.pages
expect editor.pages_tab is pages
tap panel pages.back
expect editor.modal is none
quit
