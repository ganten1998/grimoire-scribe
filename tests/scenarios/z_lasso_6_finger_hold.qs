# settings: status_mode=1
# settings: mirror_pdf=0
# settings: gesture_new_page=0
# rust-only
# The finger's way in to Select (owner ruling 2026-11-02: no panel button for Select): a finger held still
# on the page off a link (0.6 s, the link card's hold) arms a lasso, shown by a small dashed ring at the
# finger (one fast refresh, nothing else; the panel still shows the pen); drawn on without lifting, it
# selects what it goes around as the pen's lasso does, with the same bar, and the tool button shows the
# lasso; selecting is no undo step. A tap outside the selection ends it and the Select mode the hold
# entered, and draws nothing. A lasso around nothing ends Select mode at once. The fingers go through the
# touch recognizer (finger and frame), as the Kindle's do.
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
# a ballpoint word and a neighbour
pentap panel tool-picker
pentap 872 561
wait 100
expect editor.tool is ballpoint
pen curve 450 700 40 60 0.6 3
pen line 350 820 750 820 40 0.6
pen line 1100 700 1400 820 40 0.6
pen leave
wait 100
expect editor.strokes is 3
expect editor.undo_kind is add
# past the launch's whole-screen repaints (1.2 s and 3.5 s): the refreshes counted from here are the hold's
wait 3600
expect editor.select is off
expect display.presents not -1
# held still: the ring, and nothing else
finger 1 down 280 580
frame 60
wait 800
expect editor.select is lasso
expect editor.owner is select finger
expect display.presents is fast 232 532 97 97
expect editor.tool_button is ballpoint
# drawn on around the word, then lifted: selected, as the pen's lasso selects
finger 1 move 500 570
frame 20
finger 1 move 820 580
frame 20
finger 1 move 830 750
frame 20
finger 1 move 820 900
frame 20
finger 1 move 500 910
frame 20
finger 1 move 280 900
frame 20
finger 1 move 270 750
frame 20
finger 1 move 275 600
frame 20
finger 1 up 275 600
frame
wait 100
expect editor.select is selected
expect editor.selected is 2
expect editor.strokes is 3
expect editor.owner is none
expect editor.tool_button is select
expect editor.undo_kind is add
expect editor.live_matches_render is 1
shot 01-selected
# the bar is the pen lasso's: Copy says so
tap panel select.copy
wait 100
expect toast is Copied.
expect editor.clipboard is 1
# a tap outside: the selection ends, and the Select mode the hold entered with it; nothing drawn
finger 1 down 1300 1700
frame 60
finger 1 up 1300 1700
frame
wait 100
expect editor.select is off
expect editor.tool_button is ballpoint
expect editor.strokes is 3
expect editor.page is 1
# a lasso around nothing: Select mode ends with the lift
finger 1 down 1200 1500
frame 60
wait 800
expect editor.select is lasso
finger 1 move 1500 1500
frame 20
finger 1 move 1500 1800
frame 20
finger 1 move 1200 1800
frame 20
finger 1 move 1205 1510
frame 20
finger 1 up 1205 1510
frame
wait 100
expect toast is Nothing to select here.
expect editor.select is off
expect editor.owner is none
expect editor.strokes is 3
quit
