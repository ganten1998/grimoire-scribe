# settings: status_mode=1
# What's new: the bat (or the name beside it) opens the card, with Read all and Dismiss and no box
# (the owner's rule of 2026-11-05). The card shows this version's notes and ends at the last of them
# that fits whole above the signature (2.0.0's notes: all 11 upright, the first 10 sideways, the rest
# Read all's). Read all opens the list of every version's notes (2.1.0; z_about_list.qs), and its row
# for this version shows the whole notes on the sheet, on one page at the largest size that fits them
# (the owner, 2026-11-07: "fit all"; 2.0.0's notes took two pages upright and two sideways at 44 px, and
# fit at 43 px both ways, Jersey 10's next size down in Gothic), with no page arrows; Dismiss closes the
# card and the sheet. The counts are the notes' (card_notes, card_blocks, body_px, the row's version):
# every release's new notes set them again (docs/RELEASING.md, The bump commit).
# rust-only
wait 200
expect screen is library
expect library.sheet is none
expect library.ui_at.120.95 is about
expect library.ui_at.400.95 is about
tap 120 95
wait 100
expect library.sheet is about
expect library.about.full is 0
expect library.about.card_notes is 11
expect library.about.card_blocks is 11
expect library.about.card_cut is 0
shot 01-card
rotate 1
wait 200
expect library.sheet is about
expect library.about.full is 0
expect library.about.card_blocks is 10
expect library.about.card_cut is 0
shot 02-card-sideways
rotate 0
wait 200
expect library.about.card_blocks is 11
tap library about.more
wait 100
expect library.about.view is list
expect library.about.full is 1
expect library.about.card_blocks is 0
tap library about.row.2.0.0
wait 100
expect library.about.view is note
expect library.about.note is 2.0.0
expect library.about.full is 1
expect library.about.pages is 1
expect library.about.page is 1
expect library.about.body_px is 43
expect library.about.signature is 1
expect library.ui_has.about.next is 0
expect library.ui_has.about.prev is 0
shot 03-all
rotate 1
wait 200
expect library.sheet is about
expect library.about.pages is 1
expect library.about.page is 1
expect library.about.body_px is 43
expect library.about.signature is 1
expect library.ui_has.about.next is 0
shot 04-all-sideways
rotate 0
wait 200
tap library about.close
wait 100
expect library.sheet is none
tap 400 95
wait 100
expect library.sheet is about
expect library.about.full is 0
tap library about.close
wait 100
expect library.sheet is none
quit
