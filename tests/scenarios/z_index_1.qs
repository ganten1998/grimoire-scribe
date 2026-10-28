# Section names stay with page ids through insert/delete, Index opens no other page files,
# jumps leave the writing area clear, and names survive closing/reopening the notebook. Both orientations.
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index .
# rust-only
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
expect editor.page is 3
key 23 1
expect editor.pages_tab is index
expect editor.index_rows is 0
shot 01-empty-index
tap 1400 95
key 23 1
expect editor.modal is pages
# Notes
tap 175 2205
tap 1278 2205
tap 1549 1885
tap 841 1885
tap 487 1885
tap 398 2045
tap 1643 2365
expect editor.page_name is Notes
expect editor.index_rows is 1
# A failed rename keeps both the saved name and the visible name.
tap 1400 95
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
tap 125 95
pentap panel previous
pentap panel previous
expect editor.page is 1
pentap panel pages
expect editor.pages_tab is index
tap 1400 95
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
shot 02-two-sections
tap 900 435
expect editor.page is 3
expect editor.page_name is Notes
pen line 300 900 1000 950 40 0.5 0.9
expect editor.live_matches_render is 1
tap 200 74
expect editor.page is 3
shot 03-jump-clear-page
storage full
key 23 1
tap 900 300
expect editor.page is 3
expect editor.unsaved is 1
storage ok
key 23 1
tap 900 300
expect editor.page is 1
# Insert a blank note between sections.
pentap panel pages
tap 430 95
tap 396 296
tap 450 2395
expect editor.page is 2
expect editor.page_name is
expect editor.index_rows is 2
pentap panel pages
tap 850 95
shot 04-inserted-note
tap 900 435
expect editor.page is 4
expect editor.page_name is Notes
tap 200 74
expect editor.page is 4
pentap panel previous
pentap panel previous
expect editor.page is 2
# Delete the Start section. Notes still names the same physical page.
pentap panel pages
tap 430 95
tap 396 296
tap 1630 95
expect editor.index_rows is 1
tap 850 95
tap 900 300
expect editor.page is 3
expect editor.page_name is Notes
expect editor.strokes is 1
rotate 1
pentap panel pages
expect editor.pages_tab is index
shot 05-index-landscape
tap 125 95
rotate 0
pentap panel library
waitbg
tap 200 400
expect editor.page_name is Notes
expect editor.loaded_pages is 1
pentap panel pages
expect editor.pages_tab is index
expect editor.index_rows is 1
expect editor.loaded_pages is 1
shot 06-index-reopened
quit
