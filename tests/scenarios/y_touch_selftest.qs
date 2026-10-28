# rust-only
# The runner's panel names: `tap panel NAME` and `pentap panel NAME` press the panel's button where it is
# drawn now, and editor.panel_at.X.Y names the button at a point, so a scenario never has to know where the
# panel's buttons sit. A button that is disabled (Undo with nothing to undo) has no hit, as for a human
# tap; a stroke leaves the panel stale until its settle, and both settle it first, as a tap does.
# settings: mirror_pdf=0
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
# upright, no pins: where the buttons are, by name
expect editor.panel_at.1768.358 is tool-picker
expect editor.panel_at.1768.466 is eraser
expect editor.panel_at.1768.938 is
expect editor.panel_at.1768.1026 is pages
expect editor.panel_at.1768.1114 is next
expect editor.panel_at.1768.1242 is links
expect editor.panel_at.1768.1350 is paper
expect editor.panel_at.1768.1458 is display
expect editor.panel_at.1768.1566 is pen-feel
expect editor.panel_at.1768.1674 is library
expect editor.panel_at.1768.594 is
expect editor.panel_at.300.300 is
# a stroke: Undo is there, with no wait for the settle
pen line 300 700 1300 720 20 0.6 0.6
expect editor.panel_at.1768.594 is undo
tap panel undo
expect editor.strokes is 0
expect editor.panel_at.1768.594 is
expect editor.panel_at.1768.702 is redo
tap panel redo
expect editor.strokes is 1
# a stroke and the tap in one go: the line waits for the stroke to be handled
pen line 300 900 1300 920 20 0.6 0.6
pentap panel undo
expect editor.strokes is 1
pentap panel clear
expect editor.strokes is 0
# sideways
rotate 1
wait 400
pen line 300 700 1300 720 20 0.6 0.6
tap panel undo
expect editor.strokes is 0
pen line 300 900 1300 920 20 0.6 0.6
expect editor.strokes is 1
pentap panel clear
expect editor.strokes is 0
# a name that is not a button
pentap panel pen-feel
wait 300
expect screen is penset
