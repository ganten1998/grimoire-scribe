# settings: status_mode=1
# settings: shortcuts_enabled=1
# What's new's Read all opens the list of every version's notes (2.0.1, the owner's request of
# 2026-11-06: "wire read all button to a list of each UNIQUE release note, and allow users to easily
# browse the changes by version"): a row for each note, newest first, this version's marked, each with
# its version, its date (CHANGELOG.md's) and its first line. A tap on a row shows that version's notes
# whole, a page at a time, with the signature at their end; Back returns to the list; Dismiss closes
# What's new from the list and from a version's notes. Sideways the same. Each step on the sheet
# refreshes the sheet's place alone, in the sheet's refresh (ui). On a keyboard Enter opens (the card's
# Read all, then the notes of the row marked) and Esc goes back one layer (the notes to the list; the
# list and the card close). The rows are docs/whatsnew/'s notes as this build sees them (VERSION
# 2.0.0, with 2.0.1's notes written and unreleased): every release sets them again (docs/RELEASING.md,
# The bump commit).
# rust-only
wait 200
expect screen is library
tap library about
wait 100
expect library.about.view is card
expect library.about.note is 2.0.0
expect library.about.rows is 0
expect display.presents not -1
tap library about.more
wait 100
expect library.sheet is about
expect library.about.view is list
expect library.about.full is 1
expect library.about.entries is 3
expect library.about.rows is 3
expect library.about.pages is 1
expect library.about.row.1 is 2.0.1
expect library.about.row_date.1 is unreleased
expect library.about.row.2 is 2.0.0
expect library.about.row_date.2 is 2026-11-05
expect library.about.row.3 is 2.0.0-beta.1
expect library.about.row_date.3 is 2026-11-01
expect library.about.marked is 2
expect library.about.note is
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
expect library.about.signature is 1
expect library.about.rows is 0
expect library.ui_has.about.back is 1
expect library.ui_has.about.close is 1
# (the list's and the letter's place, the letter's the taller: the Library around it stays)
expect display.presents has ui 70 214 1720 2219
shot 02-letter
expect display.presents not -1
tap library about.back
wait 100
expect library.about.view is list
expect display.presents has ui 70 214 1720 2219
expect library.about.rows is 3
expect library.about.marked is 2
# Sideways: the list, and the letter on two pages, the signature on the second.
rotate 1
wait 200
expect library.sheet is about
expect library.about.view is list
expect library.about.rows is 3
expect library.about.marked is 2
shot 03-list-sideways
tap library about.row.2.0.0-beta.1
wait 100
expect library.about.view is note
expect library.about.note is 2.0.0-beta.1
expect library.about.pages is 2
expect library.about.page is 1
expect library.about.signature is 0
shot 04-letter-sideways
tap library about.next
wait 50
expect library.about.page is 2
expect library.about.signature is 1
shot 05-letter-sideways-2
tap library about.back
wait 100
expect library.about.view is list
tap library about.row.2.0.1
wait 100
expect library.about.note is 2.0.1
expect library.about.pages is 1
expect library.about.signature is 1
shot 06-unreleased-sideways
# Dismiss from a version's notes closes What's new.
tap library about.close
wait 100
expect library.sheet is none
expect library.about.view is
rotate 0
wait 200
# The keyboard: Enter opens, Esc goes back one layer.
tap library about
wait 100
expect library.about.view is card
key 28 0
wait 100
expect library.about.view is list
key 28 0
wait 100
expect library.about.view is note
expect library.about.note is 2.0.0
expect library.about.pages is 2
key 106 0
wait 50
expect library.about.page is 2
key 105 0
wait 50
expect library.about.page is 1
key 1 0
wait 100
expect library.about.view is list
key 1 0
wait 100
expect library.sheet is none
tap library about
wait 100
key 1 0
wait 100
expect library.sheet is none
# Dismiss from the list.
tap library about
wait 100
tap library about.more
wait 100
tap library about.close
wait 100
expect library.sheet is none
quit
