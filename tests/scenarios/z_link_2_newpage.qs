# rust-only
# root: z_bar_0_make
# New page... in Link to... (PHASE2 6.8): a keyboard names a page added at the end of the notebook, and the new
# link goes to it; the editor stays on its page. Cancel and a tap outside come back to Link to...; docked at the
# top with the card, Done adds an unnamed page.
wait 300
tap 200 400
wait 300
expect editor.page is 1
expect editor.pages is 5
key 38 0
pen line 1000 600 1500 800 30 0.6 0.6
expect editor.linkto.dock is bottom
expect editor.ui_at.479.2078 is linkto.chip.1
expect editor.linkto.1 is new
tap 479 2078
wait 100
expect editor.keyboard is Name the new page
expect editor.kb_dock is bottom
expect editor.linkto is pick
expect editor.modal is linkto
expect editor.link_area is 1000 600 500 200
expect editor.layers is ok
shot 01-name-the-page
# Ideas: shift, i, d, e, a, s, then Done
tap 175 2205
tap 1372 1885
tap 575 2045
tap 487 1885
tap 221 2045
tap 398 2045
expect editor.text is Ideas
tap 1643 2365
wait 100
expect editor.keyboard is
expect editor.pages is 6
expect editor.page is 1
expect toast is Linked to the new page 6.
expect editor.links is 4 4
expect editor.index_rows is 3
expect editor.linkto is 0
expect editor.undo_kind is link
expect editor.recent is 6
pen leave
wait 1100
tap 1250 700
expect editor.page is 6
expect editor.page_name is Ideas
key 48 0
expect editor.page is 1
# New page... then Cancel: back to Link to..., no page added
key 38 0
pen line 1000 1000 1500 1200 30 0.6 0.6
expect editor.linkto.chips is 6
expect editor.linkto.3 is recent p6
expect editor.ui_at.479.1938 is linkto.chip.1
tap 479 1938
expect editor.keyboard is Name the new page
tap 217 2365
wait 100
expect editor.keyboard is
expect editor.linkto is pick
expect editor.pages is 6
# and a tap outside its keyboard: the same
tap 479 1938
expect editor.keyboard is Name the new page
tap 900 300
wait 100
expect editor.keyboard is
expect editor.linkto is pick
expect editor.pages is 6
expect editor.ui_at.1530.2370 is linkto.cancel
tap 1530 2370
wait 100
expect toast is No link made.
expect editor.links is 4 4
# A lasso low on the page: the card and the keyboard dock at the top; Done with no name adds page 7 unnamed
key 38 0
pen line 300 2000 900 2150 30 0.6 0.6
expect editor.linkto.dock is top
expect editor.ui_at.479.387 is linkto.chip.1
tap 479 387
wait 100
expect editor.keyboard is Name the new page
expect editor.kb_dock is top
shot 02-name-the-page-top
# (the top dock's keys sit 1307 px higher than the bottom's: Done 1643 2365 is 1643 1058)
tap 1643 1058
wait 100
expect editor.pages is 7
expect editor.page is 1
expect toast is Linked to the new page 7.
expect editor.links is 5 5
expect editor.index_rows is 3
pentap panel library
waitbg
