# settings: status_mode=1
# settings: shortcuts_enabled=1
# What's new's Read all opens the list of every version's notes (2.1.0, the owner's request of
# 2026-11-06: "wire read all button to a list of each UNIQUE release note, and allow users to easily
# browse the changes by version"): a row for each note, newest first, this version's marked, each with
# its version, its date (CHANGELOG.md's) and its first line. A tap on a row shows that version's notes
# whole on one page (the owner, 2026-11-07: "fit all"; about.rs note_fit), with the
# signature at their end and no page arrows; Back returns to the list; Dismiss closes What's new from
# the list and from a version's notes. Sideways the same. Each step on the sheet
# refreshes the sheet's place alone, in the sheet's refresh (ui). On a keyboard Enter opens (the card's
# Read all, then the notes of the row marked) and Esc goes back one layer (the notes to the list, the
# list to the card, as the Templates sheet goes back to New notebook; the card closes). The rows are
# docs/whatsnew/'s notes as this build sees them (VERSION 2.1.0-beta.2): every release sets them again
# (docs/RELEASING.md, The bump commit).
# rust-only
wait 200
expect screen is library
tap library about
wait 100
expect library.about.view is card
expect library.about.note is 2.1.0-beta.2
expect library.about.rows is 0
expect display.presents not -1
tap library about.more
wait 100
expect library.sheet is about
expect library.about.view is list
expect library.about.full is 1
expect library.about.entries is 4
expect library.about.rows is 4
expect library.about.pages is 1
expect library.about.row.1 is 2.1.0-beta.2
expect library.about.row_date.1 is 2026-11-11
expect library.about.row.2 is 2.1.0-beta.1
expect library.about.row_date.2 is 2026-11-08
expect library.about.row.3 is 2.0.0
expect library.about.row_date.3 is 2026-11-05
expect library.about.row.4 is 2.0.0-beta.1
expect library.about.row_date.4 is 2026-11-01
expect library.about.marked is 1
expect library.about.note is
expect library.about.body_px is 0
expect library.about.lead is 0
# (Read all refreshed where the card was and where the list is, never the whole screen: the card's
# corner and width, the list's right edge, whatever their heights)
expect display.presents has ui 50 130 1740
expect library.ui_has.about.close is 1
expect library.ui_has.about.back is 0
expect library.ui_has.about.next is 0
shot 01-list
# The letter of 2.0.0-beta.1 (it has no title: the sheet's is What's new), signed, on one page upright.
expect display.presents not -1
tap library about.row.2.0.0-beta.1
wait 100
expect library.about.view is note
expect library.about.note is 2.0.0-beta.1
expect library.about.pages is 1
expect library.about.body_px is 44
expect library.about.lead is 1.3
expect library.about.signature is 1
expect library.about.rows is 0
expect library.ui_has.about.back is 1
expect library.ui_has.about.close is 1
# (the list's and the letter's place, the letter's the taller: the Library around it stays; the letter's
# sheet would end over the Library's pager, so it reaches down to the margin, 40 px above the edge)
expect display.presents has ui 70 214 1720 2226
shot 02-letter
expect display.presents not -1
tap library about.back
wait 100
expect library.about.view is list
expect display.presents has ui 70 214 1720 2226
expect library.about.rows is 4
expect library.about.marked is 1
# Sideways: the list, and the letter whole on one page too, signed: in Jersey 10's next size down (from
# 43 px), at the leading it always had, as at 44 px even its closest look takes two pages.
rotate 1
wait 200
expect library.sheet is about
expect library.about.view is list
expect library.about.rows is 4
expect library.about.marked is 1
shot 03-list-sideways
tap library about.row.2.0.0-beta.1
wait 100
expect library.about.view is note
expect library.about.note is 2.0.0-beta.1
expect library.about.pages is 1
expect library.about.page is 1
expect library.about.body_px is 43
expect library.about.lead is 1.3
expect library.about.signature is 1
expect library.ui_has.about.next is 0
expect library.ui_has.about.back is 1
shot 04-letter-sideways
tap library about.back
wait 100
expect library.about.view is list
tap library about.row.2.1.0-beta.2
wait 100
expect library.about.note is 2.1.0-beta.2
expect library.about.pages is 1
expect library.about.body_px is 44
expect library.about.signature is 1
shot 06-this-version-sideways
# Dismiss from a version's notes closes What's new.
tap library about.close
wait 100
expect library.sheet is none
expect library.about.view is
rotate 0
wait 200
# The keyboard: Enter opens, Esc goes back one layer; Right and Left do nothing on notes that fit.
tap library about
wait 100
expect library.about.view is card
key 28 0
wait 100
expect library.about.view is list
key 28 0
wait 100
expect library.about.view is note
expect library.about.note is 2.1.0-beta.2
expect library.about.pages is 1
key 106 0
wait 50
expect library.about.view is note
expect library.about.page is 1
key 105 0
wait 50
expect library.about.page is 1
key 1 0
wait 100
expect library.about.view is list
# (the list back to the card: where the list was and where the card is, as Read all, not the whole
# screen)
expect display.presents not -1
key 1 0
wait 100
expect library.about.view is card
expect library.about.note is 2.1.0-beta.2
expect library.about.rows is 0
expect library.sheet is about
expect display.presents has ui 50 130 1740
key 1 0
wait 100
expect library.sheet is none
tap library about
wait 100
key 1 0
wait 100
expect library.sheet is none
# Upright, this version's notes at the size Read all has always had (they are short); then Dismiss from
# the list.
tap library about
wait 100
tap library about.more
wait 100
tap library about.row.2.1.0-beta.2
wait 100
expect library.about.note is 2.1.0-beta.2
expect library.about.pages is 1
expect library.about.body_px is 44
expect library.about.signature is 1
shot 07-this-version
tap library about.back
wait 100
expect library.about.view is list
tap library about.close
wait 100
expect library.sheet is none
quit
