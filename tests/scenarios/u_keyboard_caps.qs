# The keyboard's shift key, a tap each: the next letter's capital (the pressed key, as before), caps on (the
# same key with a bar under its label: every letter a capital through digits, the space bar, delete and the
# symbols and accents pages), then off (2.0.0-beta.2, the owner, 2026-11-05: "next toggle shift in typing
# toggles off shift for next char then shift on which makes longer caps strings possible"). New folder's
# keyboard in Gothic, upright and sideways (u_keyboard_caps_classic.qs shoots Classic's). A shift tap
# refreshes the keyboard when the letter keys' labels change case (off to shift once, caps on to off), and
# the shift key alone from shift once to caps on (one fast refresh a tap: the touch's key and the lift's).
# On the symbols page no label changes case, so each of the three taps refreshes the shift key alone.
# rust-only
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
tap 1155 95
wait 100
expect library.sheet is keyboard
shot 01-off
expect display.presents not -1
tap 175 2205
expect display.presents is fast 20 1480 1820 980
shot 02-shift-once
expect display.presents not -1
tap 175 2205
expect display.presents is fast 50 2130 250 150
shot 03-caps-on
# NASA, the space bar, 2025, delete, 6
tap 1278 2205
tap 221 2045
tap 398 2045
tap 221 2045
tap 1018 2365
tap 310 1725
tap 1726 1725
tap 310 1725
tap 841 1725
tap 1679 2205
tap 1018 1725
expect library.text is NASA 2026
# The space bar, then the accents page (its letters in capitals), É, the letters page's T, É again, and !
# from the symbols page
tap 1018 2365
tap 747 2365
wait 100
shot 04-caps-on-accents
tap 310 1725
tap 747 2365
tap 841 1885
tap 747 2365
tap 310 1725
tap 747 2365
tap 570 2365
tap 133 1885
tap 570 2365
expect library.text is NASA 2026 ÉTÉ!
# A third tap: off, small letters
tap 1018 2365
expect display.presents not -1
tap 175 2205
expect display.presents is fast 20 1480 1820 980
shot 05-off-again
tap 929 2045
tap 1549 1885
expect library.text is NASA 2026 ÉTÉ! go
tap 1685 2365
wait 300
expect library.sheet is none
expect library.folders is NASA 2026 ÉTÉ! go
# The next keyboard opens with shift off
tap 1155 95
wait 100
expect library.sheet is keyboard
tap 221 2045
expect library.text is a
# The symbols page: each shift tap refreshes the shift key alone
tap 570 2365
expect display.presents not -1
tap 175 2205
expect display.presents is fast 50 2130 250 150
tap 175 2205
expect display.presents is fast 50 2130 250 150
tap 175 2205
expect display.presents is fast 50 2130 250 150
tap 175 2365
wait 300
expect library.sheet is none
# Sideways: shift 221 1585, a 283 1425, b 1473 1585, Cancel 221 1745
rotate 1
wait 300
tap library new-folder
wait 100
expect library.sheet is keyboard
tap 221 1585
tap 221 1585
wait 100
shot 06-caps-on-sideways
tap 283 1425
tap 1473 1585
expect library.text is AB
tap 221 1745
wait 100
expect library.sheet is none
quit
