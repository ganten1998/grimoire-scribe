# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# A Library card held 600 ms opens its menu, as its "..." does (2.2): a notebook's, the Files card's. The lift after
# the hold does nothing (the notebook does not open); a press let go sooner is a tap.
# rust-only
wait 300
expect library.items is 4
expect library.card.2.name is Beta
# the second card (Beta, a notebook): the middle of its page, upright (by its place: the card's name is no control
# once its menu covers it, and the lift must land where the press did)
touch down 929 632
wait 800
expect library.sheet is more
shot 01-held
touch up 929 632
wait 300
expect library.sheet is more
expect screen is library
tap library done
wait 300
expect library.sheet is none
# the fourth (the Files card), first of the second row
touch down 326 1569
wait 800
expect library.sheet is files-more
touch up 326 1569
wait 300
expect library.sheet is files-more
expect files.open is 0
tap library files-card.open
wait 300
expect files.open is 1
expect files.mode is place
quit
