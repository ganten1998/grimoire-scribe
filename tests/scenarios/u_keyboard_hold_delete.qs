# settings: status_mode=1
# settings: shortcuts_enabled=1
# Delete held down (2.0.0-beta.2, the owner, 2026-11-05: "hold del long enough in keyboards to quick delete", "low
# ish delay but not too low", "repeat rate high so it feels snappy", "not too high"): a tap on del deletes one at
# the touch-down, a hold under 400 ms (KB_HOLD_MS) deletes that one only, and held on it deletes one more at
# 400 ms and one every 65 ms (KB_REPEAT_MS) after, each its own fast refresh of the text line (the key, pressed
# all along, is not refreshed again), until the lift, after which nothing more is deleted or refreshed but the
# keyboard's one GL16 900 ms later. A hold that empties the field stops there: nothing more to draw. The same on
# the Find sheet, whose rows follow the text, each repeat's rows refreshed after the text line, as for a key.
# The holds timed by samples are `finger` and `frame` lines, timed by the frame clock (each `expect` ends a batch
# to see that step's refreshes); the ones with no samples at all (`touch down`, `wait`, `touch up`) are the
# screen's tick's, checked by their text, which a late machine leaves the same. Keys (portrait): del 1679 2205,
# its rect 1549 2130 261 150; the digits along y 1725 at x = 133 + 177 n; the text line 50 1510 1760 120.
# (display.presents and keyboard.draws count from their last check: a "not -1" check only starts the count)
# (the clock corner held still: its minute's refresh would land among the counted ones; and past the two
# repaints just after the start)
# rust-only
clock 1700000000
wait 3600
# The Library's keyboard (New folder), 1234567890 typed
tap 1155 95
wait 100
expect library.sheet is keyboard
tap 133 1725
tap 310 1725
tap 487 1725
tap 664 1725
tap 841 1725
tap 1018 1725
tap 1195 1725
tap 1372 1725
tap 1549 1725
tap 1726 1725
expect library.text is 1234567890
wait 1200
# A tap on del: one
tap 1679 2205
expect library.text is 123456789
wait 1200
# Held 399 ms, a sample just before the delay is up: still one
finger 1 down 1679 2205
frame 399
finger 1 move 1680 2205
frame
finger 1 up 1680 2205
frame
expect library.text is 12345678
wait 1200
# Held 630 ms, samples at 399, 400, 465, 529 and 530: the touch-down's, then one at 400, 465 and 530; none for
# the 595 that no sample or tick reached before the lift
finger 1 down 1679 2205
frame 399
finger 1 move 1680 2205
frame 1
finger 1 move 1679 2205
frame 65
finger 1 move 1680 2205
frame 64
finger 1 move 1679 2205
frame 1
finger 1 move 1680 2205
frame 100
finger 1 up 1680 2205
frame
expect library.text is 1234
# Nothing after the lift but the keyboard's GL16
expect display.presents not -1
expect keyboard.draws not -1
wait 1200
expect library.text is 1234
expect display.presents is ui 20 1480 1820 980
expect keyboard.draws is 0
# Step by step: the touch-down's delete (the text line, then the key), each repeat the text line alone and the
# keyboard drawn once, nothing once the text is gone, and the key alone at the lift
finger 1 down 1679 2205
frame 400
expect library.text is 123
expect display.presents is fast 50 1510 1760 120, fast 1549 2130 261 150
expect keyboard.draws is 1
finger 1 move 1680 2205
frame 65
expect library.text is 12
expect display.presents is fast 50 1510 1760 120
expect keyboard.draws is 1
finger 1 move 1679 2205
frame 65
expect library.text is 1
expect display.presents is fast 50 1510 1760 120
expect keyboard.draws is 1
finger 1 move 1680 2205
frame 65
expect library.text is
expect display.presents is fast 50 1510 1760 120
expect keyboard.draws is 1
finger 1 move 1679 2205
frame 400
finger 1 move 1680 2205
frame 100
expect library.text is
expect display.presents is
expect keyboard.draws is 0
finger 1 up 1680 2205
frame
expect display.presents is fast 1549 2130 261 150
wait 1200
# Held with no samples at all, the screen's tick deleting: 12345 empty at about 600 ms, then nothing more
tap 133 1725
tap 310 1725
tap 487 1725
tap 664 1725
tap 841 1725
expect library.text is 12345
wait 1200
touch down 1679 2205
wait 1200
expect library.text is
touch up 1679 2205
wait 100
expect display.presents not -1
wait 1200
expect library.text is
expect display.presents is ui 20 1480 1820 980
tap 217 2365
wait 300
expect library.sheet is none
# The Find sheet (Ctrl+F): 12 typed, then delete held: 1 at the touch-down (its row Go to page 1), the field
# empty at 400 ms (no row), each step the text line, then the rows' card 20 1312 1820 156, then (at the
# touch-down) the key
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
key 33 1
wait 300
expect editor.find is go
expect editor.keyboard is Page number or name
tap 133 1725
tap 310 1725
expect editor.text is 12
expect editor.find.rows is 1
expect editor.find.1 is There is no page 12 (1 page).
wait 1200
expect display.presents not -1
finger 1 down 1679 2205
frame 400
expect editor.text is 1
expect editor.find.1 is Go to page 1
expect display.presents is fast 50 1510 1760 120, fast 20 1312 1820 156, fast 1549 2130 261 150
finger 1 move 1680 2205
frame 100
expect editor.text is
expect editor.find.rows is 0
expect display.presents is fast 50 1510 1760 120, fast 20 1312 1820 156
finger 1 up 1680 2205
frame
expect display.presents is fast 1549 2130 261 150
wait 1200
# And by the tick: 12 again, held with no samples, empty, no rows
tap 133 1725
tap 310 1725
expect editor.text is 12
wait 1200
touch down 1679 2205
wait 1000
touch up 1679 2205
wait 100
expect editor.text is
expect editor.find.rows is 0
expect editor.find is go
quit
