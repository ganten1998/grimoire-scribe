# settings: status_mode=1
# Forward compatibility, the parts a script can reach. Pages from a newer Grimoire open read-only
# (tests/test_format.c covers those); an ordinary page never does: writing, a paper change and
# clearing all work, with no "made by a newer Grimoire" toast. Reopening the notebook (which also
# brings back pages an older version set aside) shows the page as it was.
# rust-only
wait 200
tap 965 95
wait 100
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
pen curve 300 700 40 80 0.6 4
pen line 300 900 1300 950 80 0.2 0.9
wait 700
expect toast not This page was made by a newer Grimoire. It is read-only here.
shot 01-written
# Paper: the sheet opens on an ordinary page; pick Dot grid.
pentap panel paper
wait 300
shot 02-paper-sheet
pentap 511 1035
wait 400
expect toast not This page was made by a newer Grimoire. It is read-only here.
shot 03-dot-grid
# Library and back: the page comes back as it was saved.
pentap panel library
wait 500
expect screen is library
tap 326 608
wait 500
expect screen is editor
expect toast not This page was made by a newer Grimoire. It is read-only here.
shot 04-reopened
# Clear (the panel's broom), then undo it.
pentap panel clear
wait 200
expect toast is Page cleared
shot 05-cleared
pentap panel undo
wait 400
shot 06-undone
quit
