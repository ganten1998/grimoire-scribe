# rust-only
# root: z_bar_0_make
# settings: mirror_pdf=0
# settings: eraser_mode=1
# Issue #205 beside a link: ink erased wholly inside a link's area (page 1's first, to page 3). A lasso round it
# takes the link too, as a link goes with every ink stroke inside its area (docs/beta2/LASSO-DESIGN.md 6.2), and
# the link shows (its area drawn dashed): the lasso selects both, as before 2.2. Round such ink with no link, it
# selects nothing (z_lasso_29).
wait 300
tap 200 400
wait 300
expect editor.page is 1
expect editor.links is 3 3
pen line 400 1100 800 1120 30 0.6
wait 100
eraser line 360 1110 840 1110 30
waitfor display.stale.380.1060.440.100 is 0
expect editor.strokes is 2
expect display.dark.380.1085.440.50 is 0
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
pen down 380 1050
pen move 820 1050
pen move 820 1170
pen move 380 1170
pen up 380 1060
wait 300
expect editor.select is selected
expect editor.selected is 1
expect editor.sel_links is 1
expect editor.halo is 0
quit
