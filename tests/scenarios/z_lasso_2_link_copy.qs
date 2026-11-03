# rust-only
# root: z_bar_0_make
# settings: mirror_pdf=0
# Copying a linked item copies the link and its contents exactly (docs/beta2/LASSO-DESIGN.md 2, row 5): ink
# drawn in page 1's first link area (to page 3) is lassoed with its link, copied, pasted on page 2 (the
# same notebook: the same target, a fresh seed, the area moved with the ink) and then in another notebook
# (the target rewritten n:<source>:p3, so a tap there still goes to the first notebook's page 3). Undo and
# redo take the link with the ink; a move on its own page moves the area with it, one step.
wait 300
tap 200 400
wait 300
expect editor.page is 1
expect editor.links is 3 3
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.tool is ballpoint
pen line 400 1100 800 1120 30 0.6
wait 100
expect editor.strokes is 1
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
# a lasso around the link's area and its ink: both come
pen down 250 950
pen move 600 940
pen move 950 950
pen move 960 1100
pen move 950 1250
pen move 600 1260
pen move 250 1250
pen move 240 1100
pen up 250 960
wait 100
expect editor.select is selected
expect editor.selected is 1
expect editor.sel_links is 1
shot 01-selected-link
# a move on the page: the area moves with the ink, one step, the same link
pen down 600 1100
pen move 600 1200
pen move 600 1300
pen up 600 1300
wait 100
expect editor.undo_kind is lasso
expect editor.links is 3 3
expect editor.page_links has p3@300,1200,600,200
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.page_links has p3@300,1000,600,200
expect editor.strokes is 1
pentap panel redo
wait 100
expect editor.page_links has p3@300,1200,600,200
# select it again (the undo ended the selection) and copy
pen down 250 1150
pen move 600 1140
pen move 950 1150
pen move 960 1300
pen move 950 1420
pen move 600 1425
pen move 250 1420
pen move 240 1300
pen up 250 1160
wait 100
expect editor.sel_links is 1
tap panel select.copy
wait 100
expect editor.clipboard is 1
# page 2: pasted where it was, the same target, a new link
tap panel select.done
pentap panel next
wait 300
expect editor.page is 2
expect editor.links is 0 0
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 1
expect editor.links is 1 1
expect editor.page_links is p3@300,1200,600,200
expect editor.sel_links is 1
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 0
expect editor.links is 0 0
pentap panel redo
wait 100
expect editor.strokes is 1
expect editor.links is 1 1
# another notebook: the target names the first notebook
tap panel select.done
pentap panel library
wait 500
expect screen is library
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.links is 0 0
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 1
expect editor.links is 1 1
expect editor.page_links has n:
expect editor.page_links has :p3@300,1200,600,200
expect editor.live_matches_render is 1
shot 02-pasted-elsewhere
quit
