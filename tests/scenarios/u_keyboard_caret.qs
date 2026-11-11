# The keyboard's caret, its selection and its own clipboard (2.1.0-beta.2, issue #36): New folder's keyboard in
# Gothic, upright and sideways (u_keyboard_caret_classic.qs shoots Classic's). The text line holds three keys at
# its right end: the edit key 1372 1570, the left arrow 1549 1570 and the right arrow 1726 1570; the text box is
# the rest (x 50 to 1279). An arrow moves the caret a letter at the touch-down, refreshing the text line (which
# holds the pressed key), and the lift refreshes the key alone; held, it repeats as delete does. Typing and
# delete act at the caret. A tap on the text puts the caret there, a drag across it selects. The edit key opens
# the edit page: Select all 930 1751, Cut 340 1964, Copy 930 1964, Paste 1520 1964, Start 487 2177, End 1372
# 2177, and abc 393 2365 back. What Cut and Copy take lasts the session, across keyboards, apart from the
# lasso's ink. A paired keyboard's arrows, Home and End (Shift selects) and Ctrl or Cmd with A, C, X and V do the
# same, with shortcuts off as on (the runner's keys reach the app either way; on the Kindle a keyboard is read
# only with shortcuts on), and none reaches the Library under the keyboard.
# (the clock corner held still: its minute's refresh would land among the counted ones)
# rust-only
clock 1700000000
wait 3600
expect keyboard.clipboard is
tap 1155 95
wait 100
expect library.sheet is keyboard
# "sheet", then the caret after it, as a name always typed
tap 398 2045
tap 1106 2045
tap 487 1885
tap 487 1885
tap 841 1885
expect library.caret is sheet|
wait 1200
shot 01-typed
# The left arrow: a letter back, the text line's refresh (the key pressed in it), then the key's at the lift
expect display.presents not -1
expect keyboard.draws not -1
tap 1549 1570
expect library.caret is shee|t
expect display.presents is fast 50 1510 1760 120, fast 1466 1510 167 120
expect keyboard.draws is 2
tap 1549 1570
tap 1549 1570
expect library.caret is sh|eet
wait 1200
shot 02-caret-inside
# A letter typed there goes in there; delete takes the letter before the caret
tap 1726 1725
expect library.caret is sh0|eet
tap 1679 2205
expect library.caret is sh|eet
# The right arrow; at the end it moves nothing and refreshes the key alone (its press and its lift, one refresh)
tap 1726 1570
tap 1726 1570
tap 1726 1570
tap 1726 1570
expect library.caret is sheet|
expect display.presents not -1
tap 1726 1570
expect library.caret is sheet|
expect display.presents is fast 1643 1510 167 120
# Held: one at the touch-down, one at 400 ms and one every 65 ms after, to the start, where it stops
finger 1 down 1549 1570
frame 400
finger 1 move 1550 1570
frame 65
finger 1 move 1549 1570
frame
expect library.caret is sh|eet
frame 65
finger 1 move 1550 1570
frame 65
finger 1 move 1549 1570
frame 65
finger 1 move 1550 1570
frame
finger 1 up 1550 1570
frame
expect library.caret is |sheet
# A tap on the text: right of it, the end; on the title, the start
tap 1100 1570
expect library.caret is sheet|
tap 120 1570
expect library.caret is |sheet
# A drag across it, from the title to past its end: all of it selected, white on black
drag touch 120 1570 1100 1570 4
expect library.caret is [sheet|]
wait 1200
shot 03-selected
# Typing replaces the selection
tap 1549 1885
expect library.caret is o|
tap 1679 2205
expect library.caret is |
tap 398 2045
tap 487 1885
tap 487 1885
tap 1106 2045
expect library.caret is seeh|
# The edit page, and the edit key drawn pressed while it shows
tap 1372 1570
wait 100
shot 04-edit-page
tap 930 1751
expect library.caret is [seeh|]
tap 930 1964
expect keyboard.clipboard is seeh
expect library.caret is [seeh|]
tap 1372 2177
expect library.caret is seeh|
tap 1520 1964
expect library.caret is seehseeh|
tap 487 2177
expect library.caret is |seehseeh
tap 930 1751
tap 340 1964
expect library.caret is |
expect keyboard.clipboard is seehseeh
tap 1520 1964
expect library.caret is seehseeh|
# abc goes back to the letters
tap 393 2365
wait 100
tap 221 2045
expect library.caret is seehseeha|
tap 1685 2365
wait 300
expect library.sheet is none
expect library.folders is seehseeha
# The clipboard lasts the session, across keyboards: New folder again, Paste on the edit page
tap 1155 95
wait 100
expect library.caret is |
tap 1372 1570
wait 100
tap 1520 1964
expect library.caret is seehseeh|
# A paired keyboard: Home, Shift and the right arrow, Ctrl+C, End, Cmd+V, Ctrl+A, Ctrl+X
key 102 0
expect library.caret is |seehseeh
key 106 2
key 106 2
key 106 2
key 106 2
expect library.caret is [seeh|]seeh
key 46 1
expect keyboard.clipboard is seeh
key 107 0
key 47 8
expect library.caret is seehseehseeh|
key 105 2
expect library.caret is seehseehsee[|h]
key 105 0
expect library.caret is seehseehsee|h
key 30 1
expect library.caret is [seehseehseeh|]
key 45 1
expect library.caret is |
expect keyboard.clipboard is seehseehseeh
# The right arrow on the Library under the keyboard would turn its page: still the keyboard, nothing moved
expect library.sheet is keyboard
expect library.page is 1
tap 175 2365
wait 300
expect library.sheet is none
# Sideways: the edit key 1837 950, the left arrow 2076 950, the right arrow 2315 950
rotate 1
wait 300
tap library new-folder
wait 100
expect library.sheet is keyboard
tap 283 1425
tap 1473 1585
expect library.caret is ab|
tap 2076 950
expect library.caret is a|b
wait 1200
shot 05-sideways
tap 1837 950
wait 100
shot 06-sideways-edit
tap 2315 950
expect library.caret is ab|
tap 221 1745
wait 100
expect library.sheet is none
quit
