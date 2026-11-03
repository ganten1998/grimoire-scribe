# rust-only
# generate: tests/lasso_more_fixture.py
# No surprises from the finger hold (z_lasso_6, z_lasso_8): with a full clipboard throughout, a palm never
# arms the lasso and never pastes (a contact the panel flags a palm sends nothing; one that turns into a
# palm after the hold ends without a lift); a finger held while the pen hovers, or with the pen coming in
# range before the hold time, arms nothing; the pen coming after the hold drops the armed lasso, its ring
# cleared at once, and the lift neither pastes nor selects; a thumb resting on the page while the pen
# writes elsewhere pastes nothing when it lifts, whether the pen came before or after its hold time.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 322 620
wait 1000
expect screen is editor
expect editor.strokes is 2
# the clipboard filled first: the word lassoed with a finger and copied, then a tap outside
touch down 420 600
wait 800
touch move 620 590
touch move 830 600
touch move 840 710
touch move 830 830
touch move 620 840
touch move 420 830
touch move 410 710
touch up 420 610
wait 100
expect editor.selected is 2
tap panel select.copy
wait 100
expect editor.clipboard is 1
tap 1300 2000
wait 100
expect editor.select is off
# a palm (MT_TOOL_PALM) held still: nothing reaches the page, nothing arms, nothing pastes
finger 1 down 900 1700 0 2
frame 60
wait 800
expect editor.select is off
expect editor.owner is none
finger 1 up 900 1700
frame
wait 100
expect editor.strokes is 2
expect editor.select is off
# a finger held, armed, then called a palm: its stream ends without a lift; no paste, ring gone
finger 1 down 900 1700
frame 60
wait 800
expect editor.select is lasso
finger 1 move 900 1700 0 2
frame 20
expect editor.select is off
expect editor.owner is none
finger 1 up 900 1700
frame
wait 100
expect editor.strokes is 2
expect editor.select is off
# the pen hovering: a finger held still arms nothing (it is only held), and its lift pastes nothing
pen hover 300 300
touch down 900 1700
wait 800
expect editor.owner is link finger
expect editor.select is off
touch up 900 1700
wait 100
expect editor.select is off
expect editor.strokes is 2
pen leave
wait 600
# the pen coming in range before the hold time, and gone again: still nothing
touch down 900 1700
wait 200
pen hover 300 300
pen leave
wait 600
expect editor.select is off
touch up 900 1700
wait 100
expect editor.strokes is 2
wait 600
# the pen coming after the hold: the armed lasso drops, its ring cleared; the lift pastes nothing
expect display.presents not -1
touch down 900 1700
wait 800
expect editor.select is lasso
expect display.presents is fast 852 1652 97 97
pen hover 300 300
expect editor.select is off
expect editor.owner is none
expect display.presents has ui 852 1652 97 97
touch move 1200 1700
touch up 1200 1700
wait 100
expect editor.select is off
expect editor.strokes is 2
expect editor.undo_kind is
pen leave
wait 600
# a thumb resting while the pen writes elsewhere (the pen before its hold time): no paste at its lift
touch down 1700 2200
wait 200
pen line 300 1200 900 1250 30 0.6
pen leave
wait 800
expect editor.select is off
touch up 1700 2200
wait 100
expect editor.strokes is 3
expect editor.select is off
expect editor.undo_kind is add
wait 600
# the same with the pen after the thumb's hold time: the armed lasso drops at the pen's first event
touch down 1700 2200
wait 800
expect editor.select is lasso
pen line 300 1400 900 1450 30 0.6
expect editor.select is off
pen leave
wait 300
touch up 1700 2200
wait 100
expect editor.strokes is 4
expect editor.select is off
expect editor.undo_kind is add
expect editor.live_matches_render is 1
quit
