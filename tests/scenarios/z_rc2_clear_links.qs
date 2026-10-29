# rust-only
# root: z_bar_0_make
# settings: mirror_pdf=0
# settings: shape_quads=2
wait 300
tap 200 400
wait 200
expect editor.strokes is 0
expect editor.links is 3 3
# Clear is active on an ink-free page with link areas.
tap panel clear
expect editor.links is 0 0
expect editor.undo_kind is clear
tap panel undo
expect editor.links is 3 3
expect editor.strokes is 0
# One undo restores ink and links together.
pen line 350 800 900 820 12 0.6 0.6
pen leave
wait 300
expect editor.strokes is 1
tap panel clear
expect editor.links is 0 0
expect editor.strokes is 0
tap panel undo
expect editor.links is 3 3
expect editor.strokes is 1
tap panel redo
expect editor.links is 0 0
expect editor.strokes is 0
# The toolbar starts an ink-free, snapped link area, including a target of The Index.
tap panel links
expect editor.link_mode is 1
pen down 500 900 0.6
pen move 1100 900 0.6
pen move 1100 1300 0.6
pen move 500 1300 0.6
pen move 500 900 0.6
wait 700
expect editor.link_shape is square
expect editor.strokes is 0
shot 01-snapped-link-selection
pen up 500 900
pen leave
expect editor.linkto is pick
expect editor.link_area is 550 850 500 500
tap panel linkto.index
expect editor.link.1 is index
expect editor.links is 1 1
expect editor.strokes is 0
wait 1100
swipe 2 930 1400 0 -400
wait 200
expect editor.bar is open
tap panel links
expect editor.bar is 0
expect editor.link_mode is 1
tap panel links
expect editor.link_mode is 0
tap panel clear
expect editor.links is 0 0
tap panel next
tap panel previous
expect editor.links is 0 0
tap panel library
waitbg
tap 200 400
wait 300
expect editor.links is 0 0
expect editor.strokes is 0
