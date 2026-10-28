# rust-only
# More entries than a screenful: a page of 13 links to 13 pages gets the pager next to the X, three screens of six
# (upright and sideways alike); its arrows turn the screenful and only they redraw the chips, and the card does
# not change size as they turn.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/index-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
pentap panel next
expect editor.pages is 14
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
expect editor.page is 1
# Thirteen links down the page, each to the next page: key 38 0 (add-link), the box, then the page picked
key 38 0
pen line 300 300 900 340 30 0.6 0.6
expect editor.picking is new
tap 703 340
expect editor.links is 1 1
key 38 0
pen line 300 450 900 490 30 0.6 0.6
expect editor.picking is new
tap 1155 340
expect editor.links is 2 2
key 38 0
pen line 300 600 900 640 30 0.6 0.6
expect editor.picking is new
tap 1607 340
expect editor.links is 3 3
key 38 0
pen line 300 750 900 790 30 0.6 0.6
expect editor.picking is new
tap 251 680
expect editor.links is 4 4
key 38 0
pen line 300 900 900 940 30 0.6 0.6
expect editor.picking is new
tap 703 680
expect editor.links is 5 5
key 38 0
pen line 300 1050 900 1090 30 0.6 0.6
expect editor.picking is new
tap 1155 680
expect editor.links is 6 6
key 38 0
pen line 300 1200 900 1240 30 0.6 0.6
expect editor.picking is new
tap 1607 680
expect editor.links is 7 7
key 38 0
pen line 300 1350 900 1390 30 0.6 0.6
expect editor.picking is new
tap 251 1020
expect editor.links is 8 8
key 38 0
pen line 300 1500 900 1540 30 0.6 0.6
expect editor.picking is new
tap 703 1020
expect editor.links is 9 9
key 38 0
pen line 300 1650 900 1690 30 0.6 0.6
expect editor.picking is new
tap 1155 1020
expect editor.links is 10 10
key 38 0
pen line 300 1800 900 1840 30 0.6 0.6
expect editor.picking is new
tap 1607 1020
expect editor.links is 11 11
key 38 0
pen line 300 1950 900 1990 30 0.6 0.6
expect editor.picking is new
tap 251 1360
expect editor.links is 12 12
key 38 0
pen line 300 2100 900 2140 30 0.6 0.6
expect editor.picking is new
tap 703 1360
expect editor.links is 13 13
expect editor.page is 1
# The bar: 13 entries in three screens (after the pen window: it hovers after a stroke until it leaves)
pen leave
wait 1100
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.entries is 13
expect editor.bar.pager is 1/3
expect editor.bar.rect is 20 1664 1820 796
shot 01-pager-first
# › twice reaches the third screen, which holds the thirteenth entry alone; ‹ comes back
tap 1627 1766
wait 300
expect editor.bar.pager is 2/3
expect editor.bar.rect is 20 1664 1820 796
tap 1627 1766
wait 300
expect editor.bar.pager is 3/3
shot 02-pager-last
tap 1627 1766
wait 300
expect editor.bar.pager is 3/3
tap 1399 1766
wait 300
expect editor.bar.pager is 2/3
tap 1399 1766
wait 300
expect editor.bar.pager is 1/3
tap 1399 1766
wait 300
expect editor.bar.pager is 1/3
expect editor.bar.13 is p14
expect editor.bar.entries is 13
expect editor.bar is open
