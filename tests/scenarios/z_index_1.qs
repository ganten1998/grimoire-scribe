# settings: status_mode=1
# Names stay attached to physical page ids through insert/delete and reopening. Graph
# navigation and Pages management read metadata without loading other pages, in both orientations.
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/link-keys .
# rust-only
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
expect editor.page is 3
pentap panel pages
expect editor.map is graph
expect editor.map.list is 0
expect editor.index_rows is 0
expect editor.map.nodes is 3
shot 01-unnamed-graph
tap panel map.back
# Name the open page through its shortcut; Done returns to the notebook.
key 49 0
expect editor.modal is pages
expect editor.keyboard is Page name
tap 175 2205
tap 1278 2205
tap 1549 1885
tap 841 1885
tap 487 1885
tap 398 2045
tap 1643 2365
expect editor.page_name is Notes
expect editor.index_rows is 1
expect editor.modal is none
# A failed rename keeps both the saved name and the visible name.
key 49 0
expect editor.text is Notes
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 569 2205
storage full
tap 1643 2365
expect editor.page_name is Notes
expect editor.index_rows is 1
expect toast is Could not save the page name.
storage ok
pentap panel previous
pentap panel previous
expect editor.page is 1
key 49 0
# Start
tap 175 2205
tap 398 2045
tap 841 1885
tap 221 2045
tap 664 1885
tap 841 1885
tap 1643 2365
expect editor.page_name is Start
expect editor.index_rows is 2
pentap panel pages
expect editor.map is graph
shot 02-two-sections
tap map-node 3
tap map-node 3
expect editor.page is 3
expect editor.page_name is Notes
expect editor.modal is none
pen line 300 900 1000 950 40 0.5 0.9
expect editor.live_matches_render is 1
shot 03-jump-clear-page
# A failed save must prevent Graph navigation from discarding this ink.
storage full
pentap panel pages
tap map-node 1
tap map-node 1
expect editor.page is 3
expect editor.unsaved is 1
storage ok
pentap panel pages
tap map-node 1
tap map-node 1
expect editor.page is 1
# Insert after Start. Page id 3 must still open Notes at its new page number 4.
pentap panel pages
tap panel map.pages
expect editor.modal is pages
tap panel pages.new
expect editor.page is 2
expect editor.page_name is
expect editor.index_rows is 2
pentap panel pages
expect editor.map is graph
shot 04-inserted-note
tap map-node 3
tap map-node 3
expect editor.page is 4
expect editor.page_name is Notes
expect editor.strokes is 1
pentap panel previous
pentap panel previous
expect editor.page is 2
# Delete Start, selecting only its thumbnail checkbox in management.
pentap panel pages
tap panel map.pages
tap 396 296
expect editor.ticked is 1
tap panel pages.delete
expect editor.index_rows is 1
tap panel pages.graph
tap map-node 3
tap map-node 3
expect editor.page is 3
expect editor.page_name is Notes
expect editor.strokes is 1
rotate 1
pentap panel pages
expect editor.map is graph
expect editor.map.nodes is 3
shot 05-graph-landscape
tap panel map.back
rotate 0
pentap panel library
waitbg
tap 200 400
expect editor.page_name is Notes
expect editor.loaded_pages is 1
pentap panel pages
expect editor.map is graph
expect editor.index_rows is 1
expect editor.loaded_pages is 1
tap panel map.pages
expect editor.modal is pages
expect editor.loaded_pages is 1
shot 06-management-reopened
quit
