# rust-only
# generate: tests/lasso_more_fixture.py
# Cut and paste (docs/beta2/LASSO-DESIGN.md 6.1, 6.7, 10): Select mode shows on the panel (the tool button
# is the lasso); Cut says so; a paste after a Cut lands where the group was (the originals are gone), one
# after a Copy 3 mm off; a pen or a finger held still on empty page pastes centred there; a group cut on page 1
# pastes on page 2 where it was; on a read-only page Select and Copy work and Cut, Delete, a drag and Paste
# are refused with nothing changed; a group wider than another notebook's page is scaled to fit there.
wait 300
tap 322 620
wait 1000
expect screen is editor
expect editor.strokes is 2
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
expect editor.tool_button is select
# the word: lassoed, cut, pasted back where it was
pen down 420 600
pen move 620 590
pen move 830 600
pen move 840 710
pen move 830 830
pen move 620 840
pen move 420 830
pen move 410 710
pen up 420 610
wait 100
expect editor.select is selected
expect editor.selected is 2
expect editor.sel_box is 495 695 331 131
tap panel select.cut
wait 100
expect toast is
expect editor.clipboard is 1
expect editor.strokes is 0
tap panel select.paste
wait 100
expect editor.strokes is 2
expect editor.sel_box is 495 695 331 131
expect editor.live_matches_render is 1
# copied: the paste reads as a copy, 3 mm off the originals
tap panel select.copy
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 4
expect editor.sel_box is 530 730 331 131
pentap panel undo
wait 100
expect editor.strokes is 2
expect editor.select is armed
# the pen held still on empty page: a paste centred there (the rest of the contact does nothing)
pen down 900 1700
wait 800
pen move 905 1702
pen up 905 1702
wait 100
expect editor.select is selected
expect editor.strokes is 4
expect editor.sel_box is 796 1750 331 131
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 2
# a finger held there: the same, once the pen has gone (a finger's hold with the pen in range arms nothing)
pen leave
wait 600
touch down 900 1700
wait 800
touch up 900 1700
wait 100
expect editor.select is selected
expect editor.strokes is 4
expect editor.sel_box is 796 1750 331 131
pentap panel undo
wait 100
expect editor.strokes is 2
# cut on page 1, paste on page 2: where it was (on screen there too)
pen down 420 600
pen move 620 590
pen move 830 600
pen move 840 710
pen move 830 830
pen move 620 840
pen move 420 830
pen move 410 710
pen up 420 610
wait 100
expect editor.selected is 2
tap panel select.cut
wait 100
expect editor.strokes is 0
pentap panel next
wait 300
expect editor.page is 2
expect editor.select is armed
tap panel select.paste
wait 100
expect editor.strokes is 2
expect editor.sel_box is 495 695 331 131
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.strokes is 0
pentap panel redo
wait 100
expect editor.strokes is 2
pentap panel previous
wait 300
expect editor.page is 1
expect editor.strokes is 0
# a read-only page: Select and Copy work; Cut, Delete, a drag and Paste are refused, nothing changes
pentap panel next
wait 300
pentap panel next
wait 300
expect editor.page is 3
expect editor.strokes is 1
pen down 15 1350
pen move 900 1340
pen move 1850 1350
pen move 1855 1410
pen move 1850 1520
pen move 900 1530
pen move 15 1520
pen move 10 1410
pen up 15 1360
wait 100
expect editor.select is selected
expect editor.selected is 1
tap panel select.copy
wait 100
expect toast is
expect editor.clipboard is 1
tap panel select.cut
wait 100
expect toast has read-only
expect editor.strokes is 1
expect editor.select is selected
tap panel select.delete
wait 100
expect editor.strokes is 1
pen down 900 1430
pen move 900 1600
pen move 900 1800
pen up 900 1800
wait 100
expect editor.strokes is 1
expect editor.sel_box is 35 1495 1911 71
tap panel select.paste
wait 100
expect toast has read-only
expect editor.strokes is 1
expect editor.undo_kind is
shot 02-readonly
# the wide line pasted into a notebook of a smaller page size: scaled to fit (design row 4)
tap panel select.done
pentap panel library
wait 500
expect screen is library
tap 930 620
wait 1000
expect screen is editor
expect editor.strokes is 0
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 1
expect editor.sel_box is 0 1205 1860 70
expect editor.live_matches_render is 1
tap panel select.done
wait 100
expect editor.tool_button not select
shot 03-scaled
quit
