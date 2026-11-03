# rust-only
# generate: tests/lasso_more_fixture.py
# The finger hold's paste (owner ruling 2026-11-02): a finger held still on empty page and lifted without
# moving pastes the clipboard centred there, selected, one undo step; with nothing on the clipboard the
# same hold does nothing but draw and clear its ring (two refreshes in all). A selection on the page: a
# finger held outside it drops it and starts again. On a read-only page the finger's lasso and Copy work,
# and its paste is refused as the pen's is, nothing changed.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 322 620
wait 1000
expect screen is editor
expect editor.strokes is 2
expect editor.clipboard is 0
# past the launch's whole-screen repaints: the refreshes counted are the hold's
wait 2600
expect display.presents not -1
touch down 900 1700
wait 800
expect editor.select is lasso
touch up 900 1700
wait 100
expect editor.select is off
expect editor.strokes is 2
expect editor.undo_kind is
expect display.presents is fast 852 1652 97 97, ui 852 1652 97 97
# the word lassoed with a finger, copied from the bar
touch down 420 600
wait 800
expect editor.select is lasso
touch move 620 590
touch move 830 600
touch move 840 710
touch move 830 830
touch move 620 840
touch move 420 830
touch move 410 710
touch up 420 610
wait 100
expect editor.select is selected
expect editor.selected is 2
expect editor.sel_box is 495 695 331 131
tap panel select.copy
wait 100
expect toast is Copied.
expect editor.clipboard is 1
# held and lifted outside the selection: it goes, and the paste lands centred on the hold, selected
touch down 900 1700
wait 800
expect editor.select is lasso
touch up 900 1700
wait 100
expect editor.select is selected
expect editor.strokes is 4
expect editor.sel_box is 796 1750 331 131
expect editor.undo_kind is lasso
expect editor.tool_button is select
expect editor.live_matches_render is 1
shot 01-pasted
pentap panel undo
wait 100
expect editor.strokes is 2
expect editor.undo_kind is
pentap panel next
wait 300
pentap panel next
wait 300
expect editor.page is 3
expect editor.select is off
expect editor.strokes is 1
# (the pen gone: a finger's hold with the pen in range arms nothing, z_lasso_9)
pen leave
wait 600
# a read-only page: the finger's lasso and Copy work, its paste is refused and nothing changes
touch down 15 1350
wait 800
expect editor.select is lasso
touch move 900 1340
touch move 1850 1350
touch move 1855 1410
touch move 1850 1520
touch move 900 1530
touch move 15 1520
touch move 10 1410
touch up 15 1360
wait 100
expect editor.select is selected
expect editor.selected is 1
tap panel select.copy
wait 100
expect toast is Copied.
touch down 900 1900
wait 800
expect editor.select is lasso
touch up 900 1900
wait 100
expect toast has read-only
expect editor.strokes is 1
expect editor.select is off
expect editor.undo_kind is
shot 02-readonly
quit
