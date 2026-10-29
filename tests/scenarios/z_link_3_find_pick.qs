# rust-only
# root: z_bar_0_make
# Find in pick mode (PHASE2 7.2): Link to...'s Search finds where a new link goes, by name or number, over the
# page: no page change, nothing on the back list. Cancel comes back to Link to...; for an area low on the page the
# keyboard docks at the top, below the clock corner, with the rows under it.
wait 300
tap 200 400
wait 300
expect editor.links is 3 3
key 38 0
pen line 1000 600 1500 800 30 0.6 0.6
expect editor.ui_at.330.2370 is linkto.search
tap 330 2370
wait 100
expect editor.find is pick
expect editor.modal is find
expect editor.linkto is pick
expect editor.kb_dock is bottom
expect editor.keyboard is Page number or name
expect editor.link_area is 1000 600 500 200
expect editor.layers is ok
shot 01-find-pick
# arb: a, r, b
tap 221 2045
tap 664 1885
tap 1101 2205
expect editor.find.rows is 1
expect editor.find.1 is Árbol
tap 930 1390
wait 100
expect editor.find is 0
expect editor.linkto is 0
expect toast is Linked to Árbol.
expect editor.links is 4 4
expect editor.page is 1
expect editor.back is 0 0
expect editor.undo_kind is link
# A page number: Link to page 2, and Done takes it
key 38 0
pen line 1000 1000 1500 1200 30 0.6 0.6
tap 330 2370
wait 100
tap 310 1725
expect editor.find.1 is Link to page 2
tap 1643 2365
wait 100
expect toast is Linked to page 2.
expect editor.links is 5 5
expect editor.page is 1
expect editor.back is 0 0
# Search, then Cancel: back to Link to...
key 38 0
pen line 1000 600 1500 800 30 0.6 0.6
tap 330 2370
wait 100
expect editor.find is pick
tap 217 2365
wait 100
expect editor.find is 0
expect editor.linkto is pick
expect editor.modal is linkto
expect editor.layers is ok
# and a tap outside it: the same
tap 330 2370
wait 100
expect editor.find is pick
tap 900 300
wait 100
expect editor.find is 0
expect editor.linkto is pick
expect editor.ui_at.1530.2370 is linkto.cancel
tap 1530 2370
wait 100
expect toast is No link made.
expect editor.links is 5 5
# A lasso low on the page: Link to... and the keyboard dock at the top, the rows under the keyboard
key 38 0
pen line 300 2000 900 2150 30 0.6 0.6
expect editor.linkto.dock is top
expect editor.linkto.rect is 20 173 1820 736
expect editor.ui_at.330.819 is linkto.search
tap 330 819
wait 100
expect editor.find is pick
expect editor.kb_dock is top
# (the top dock's keys sit 1307 px higher: a 221 738, r 664 578, e 487 578)
tap 221 738
tap 664 578
tap 487 578
expect editor.find.1 is Arena
expect editor.layers is ok
shot 02-find-pick-top
tap 930 1247
wait 100
expect toast is Linked to Arena.
expect editor.links is 6 6
expect editor.page is 1
pentap panel library
waitbg
