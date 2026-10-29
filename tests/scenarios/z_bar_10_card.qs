# settings: status_mode=1
# rust-only
# root: z_bar_0_make
# A press held on a Links bar row (PHASE2 8.10, D1): the long-press card for all of the entry's links on the page,
# above the bar in the chip's column, with the heavy border on each; the bar stays under it. Its rows act on all
# of them in one step: Delete 2 links (one Undo brings both back), Change where they go... (both to the Index,
# one Undo). While the bar looks at another page, the hold looks at the bar's own page again first, in the page
# area's GL16 (no DU strips). Two fingers down close the card and the bar together.
wait 3700
tap 200 400
wait 300
expect editor.page is 1
expect editor.links is 3 3
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.1 is p3 x2
expect display.presents not -1
touch down 479 2218
wait 800
expect editor.link_card is 1
expect editor.link_card.links is 2
expect editor.link_card.rows is rename retarget delete
expect editor.link_card.border is 2
expect editor.link_card.title is Árbol
expect editor.bar is open
expect editor.modal is card
expect editor.link_card.rect is 40 1320 780 608
# the chip let go (DU), then each box's four border strips on their own, then the card and the chip (one GL16)
expect display.presents has fast 40 2156 878 124, fast 289 989 622 11, fast 289 1200 622 11, fast 289 1000 11 200, fast 900 1000 11 200, fast 289 1789 622 11
touch up 479 2218
expect editor.link_card is 1
expect editor.bar is open
shot 01-card-from-bar
# Delete 2 links: both go, one step
expect editor.ui_at.430.1773 is card.delete
tap 430 1773
wait 100
expect editor.links is 1 1
expect toast is Links deleted. Undo brings them back.
expect editor.link_card is 0
expect editor.bar is open
expect editor.bar.entries is 1
expect editor.bar.1 is p2
tap 1715 2026
wait 300
expect editor.bar is 0
pentap panel undo
wait 300
expect editor.links is 3 3
expect editor.undo_kind is
expect editor.redo_kind is link
# Looking at page 2, then a hold on entry 1: the bar looks at its own page again, in the page area's GL16
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
tap 1381 2218
wait 300
expect editor.bar.peek is 2
expect display.presents not -1
touch down 479 2218
wait 800
touch up 479 2218
expect editor.link_card is 1
expect editor.bar.peek is 0
expect editor.page is 1
expect display.presents is fast 40 2156 878 124, ui 0 0 1860 2460, ui 40 1320 878 960
shot 02-card-after-peek
# Change where they go...: both to the Index in one step, one Undo brings both back
expect editor.ui_at.430.1643 is card.retarget
tap 430 1643
wait 100
expect editor.bar is 0
expect editor.linkto is pick
expect editor.linkto.rect is 20 173 1820 596
expect editor.linkto.3 is p3 current
expect editor.ui_at.1381.387 is linkto.chip.2
tap 1381 387
wait 100
expect toast is Linked to the Index.
expect editor.bar is open
expect editor.bar.entries is 2
expect editor.bar.1 is index
expect editor.bar.2 is p2
expect editor.undo_kind is link
tap 1715 2026
wait 300
# (links.ini lists a page's links by seed, made from the clock: where they go is checked by a finger's tap)
pen leave
wait 1100
tap 600 1850
expect editor.modal is pages
expect editor.pages_tab is index
tap 125 95
wait 300
tap 600 1450
expect editor.page is 2
pentap panel previous
expect editor.page is 1
pentap panel undo
wait 300
expect toast is Link change undone.
expect editor.links is 3 3
pen leave
wait 1100
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
pen leave
wait 1100
tap 600 1850
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# Two fingers down while the card is up over the bar: the card and the bar close together
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
touch down 479 2218
wait 800
touch up 479 2218
expect editor.link_card is 1
swipe 2 1300 300 0 400
wait 300
expect editor.link_card is 0
expect editor.bar is 0
expect editor.modal is none
