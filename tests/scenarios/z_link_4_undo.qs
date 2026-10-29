# rust-only
# root: z_bar_0_make
# Link edits are undo steps (PHASE2 4): the card's Delete takes a link away and Undo brings it back whole (it
# goes where it went), Redo takes it again; link steps and ink come off one stack in order; a save that fails
# changes nothing and pushes no step, and an undo whose save fails is dropped; with the eye on, an undo shows in
# the boxes (one GL16) and says nothing.
wait 300
tap 200 400
wait 300
expect editor.links is 3 3
expect editor.strokes is 0
# Hold the link to page 2, Delete
touch down 600 1450
wait 800
touch up 600 1450
expect editor.link_card is 1
expect editor.ui_at.679.2118 is card.delete
tap 679 2118
expect editor.links is 2 2
expect editor.link_card is 0
expect toast is Link deleted. Undo brings it back.
expect editor.undo_kind is link
pentap panel undo
wait 300
expect editor.links is 3 3
expect editor.redo_kind is link
pen leave
wait 1100
tap 600 1450
expect editor.page is 2
pentap panel previous
expect editor.page is 1
pentap panel redo
wait 300
expect editor.links is 2 2
pentap panel undo
wait 300
expect editor.links is 3 3
# Interleaved with ink: a stroke, then the link at 600 1100 deleted through the card; undo brings the link
# back, the next undo takes the stroke away
pen line 1000 600 1400 700 30 0.6 0.6
expect editor.strokes is 1
pen leave
wait 1100
touch down 600 1100
wait 800
touch up 600 1100
expect editor.ui_at.679.1818 is card.delete
tap 679 1818
expect editor.links is 2 2
pentap panel undo
wait 300
expect editor.links is 3 3
expect editor.strokes is 1
expect editor.undo_kind is add
pentap panel undo
wait 300
expect editor.strokes is 0
expect editor.links is 3 3
# A save that fails: nothing changes and no step is pushed (the page's own save is done first)
pen leave
wait 2500
storage full
touch down 600 1450
wait 800
touch up 600 1450
tap 679 2118
expect toast is Could not save the link.
expect editor.links is 3 3
expect editor.undo_kind is
storage ok
wait 3100
touch down 600 1450
wait 800
touch up 600 1450
tap 679 2118
expect editor.links is 2 2
expect editor.undo_kind is link
# an undo whose save fails: the link stays deleted and the step is gone
storage full
pentap panel undo
wait 300
expect toast is Could not save the links: that change was not undone.
expect editor.links is 2 2
expect editor.redo_kind is
expect editor.undo_kind is
storage ok
# The eye on: a link undo shows in the boxes, one GL16 there, and no toast
key 30 0
wait 3100
pen leave
wait 1100
touch down 600 1100
wait 800
touch up 600 1100
tap 679 1818
expect editor.links is 1 1
wait 3100
expect display.presents not -1
pentap panel undo
wait 300
expect editor.links is 2 2
expect toast is
# (the panel's refresh, Redo enabled now, joins the boxes' one: a GL16 over the page, never a Full or a DU)
expect display.presents is fast 1718 652 100 100, ui 297 220 1535 1683
