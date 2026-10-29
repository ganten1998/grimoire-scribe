# rust-only
# root: z_bar_0_make
# Smallest wins (PHASE2 3.5): a big link drawn around the root's link to page 3 (300 1000 to 900 1200) and made
# after it, to page 5 (Arena). A tap inside the small one follows the small one; elsewhere in the big one, the big.
wait 300
tap 200 400
wait 300
expect editor.links is 3 3
key 38 0
pen line 200 900 1600 1300 30 0.6 0.6
expect editor.link_area is 200 900 1400 400
expect editor.linkto.4 is p5
expect editor.ui_at.1381.2218 is linkto.chip.4
tap 1381 2218
wait 100
expect toast is Linked to Arena.
expect editor.links is 4 4
pen leave
wait 1100
tap 600 1100
expect editor.page is 3
key 48 0
expect editor.page is 1
tap 1500 1250
expect editor.page is 5
key 48 0
expect editor.page is 1
pentap panel library
waitbg
