# settings: status_mode=1
# settings: theme=classic
# What's new in Classic, 2.0.0's notes on Read all's sheet (2.1.0; the owner, 2026-11-07: "The note
# reading itself isn't polished enough"). Upright they fit at 44 px in the look they always had.
# Sideways even the closest look at 44 px (the leading 1.15, the gaps 60% of theirs) takes two pages:
# the type steps down to the largest size whose closest look fits, 41.5 px, at the leading 1.15
# (fitting by size alone set them at 38.5 px, and the room left under them). Their sheet would then
# start across the Library's top bar's buttons: it starts where the tallest sheet does, at the margin
# above them, and reaches down to the margin under the pager. The
# step from the list refreshes the list's place and the sheet's alone. The counts are the notes'
# (body_px, lead): every release's new notes set them again (docs/RELEASING.md, The bump commit).
# rust-only
wait 200
expect screen is library
tap library about
wait 100
tap library about.more
wait 100
expect library.about.view is list
tap library about.row.2.0.0
wait 100
expect library.about.view is note
expect library.about.pages is 1
expect library.about.body_px is 44
expect library.about.lead is 1.3
expect library.about.signature is 1
shot 01-note
tap library about.back
wait 100
rotate 1
wait 200
expect library.about.view is list
expect display.presents not -1
tap library about.row.2.0.0
wait 100
expect library.about.view is note
expect library.about.pages is 1
expect library.about.body_px is 41.5
expect library.about.lead is 1.15
expect library.about.signature is 1
expect library.ui_has.about.next is 0
expect display.presents has ui 340 40 1800 1780
shot 02-note-sideways
tap library about.close
wait 100
expect library.sheet is none
quit
