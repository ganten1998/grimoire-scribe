# rust-only
# generate: tests/move_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# New page in the Pages view (owner, 2026-11-03): a tap adds one page at once, after the open page, and the
# view stays (owner, later the same day: "make the page add button not enter the page when its made"), the
# new tile marked "(new)", a toast saying which page it is, and Undo offered as for a batch of one. Held (a held tile's 600 ms) it opens the count pad instead: the number pad Move to... and Go to
# page use, with - and + under the digits and Done labelled with the count ("Add 12 pages", off at 0). Done adds
# that many blank pages in one go where New page puts one, the view staying on the first of them, and the
# bottom row's Undo (in Move to...'s place) takes them back while they are still blank. Each key refreshes the
# count line and Done, each on its own fast refresh, and the key.
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting; a slow
# runner reached the 3.5 s one inside a counted stretch)
wait 3600
clock 1700000000
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5
tap panel pages
waitbg
wait 200
# (the + the script holds: the bottom row's first button)
expect editor.pages.at.new is 394 2395
# A tap: one page at once, no pad, not opened
tap panel pages.new
wait 300
expect editor.modal is pages
expect editor.pages.adding is 0
expect editor.page_ids is 1,6,2,3,4,5
expect editor.page is 1
expect toast is Added 1 page: it is page 2.
expect editor.tile.1 is one
expect editor.tile.2 is Page 2 (new)
expect editor.tile.3 is two
expect editor.pages.undo.kind is add
# Opened, it is the open page: its Undo is no longer offered, nor its mark
tap panel pages.page.2
tap panel pages.page.2
wait 300
expect editor.modal is none
expect editor.page is 2
tap panel pages
wait 200
expect editor.tile.2 is Page 2 (open)
expect editor.pages.adding is 0
expect editor.pages.undo is 0
# Held: the count pad, at 1 (highlighted: the first digit typed replaces it), Done on
touch down 394 2395
wait 700
expect editor.pages.adding is 1
touch up 394 2395
wait 100
expect editor.pages.adding is 1
expect editor.pages.keyboard is Pages to add
expect editor.pages.text is 1
expect editor.pages.add.done is Add 1 page
expect editor.pages.add.done.on is 1
expect editor.page_ids is 1,6,2,3,4,5
shot 01-pad
# del clears it: Done off, and Done at 0 does nothing
tap panel pages.key.del
wait 100
expect editor.pages.text is
expect editor.pages.add.done.on is 0
tap panel pages.key.done
wait 100
expect editor.pages.adding is 1
expect editor.pages.keyboard is Pages to add
# Type 12: the count, and Done says so
expect display.presents not -1
tap panel pages.key.1
wait 100
expect editor.pages.add.done is Add 1 page
expect display.presents is fast 50 1340 1760 120, fast 50 2310 1760 150, fast 50 1480 576 150
tap panel pages.key.2
wait 100
expect editor.pages.text is 12
expect editor.pages.add.done is Add 12 pages
expect editor.pages.add.done.on is 1
# - and +: one less, one more
tap panel pages.key.dec
wait 100
expect editor.pages.text is 11
tap panel pages.key.inc
tap panel pages.key.inc
wait 100
expect editor.pages.text is 13
tap panel pages.key.dec
wait 100
expect editor.pages.text is 12
shot 02-twelve
tap panel pages.key.done
wait 300
expect editor.pages.adding is 0
expect editor.modal is pages
expect editor.page_ids is 1,6,7,8,9,10,11,12,13,14,15,16,17,18,2,3,4,5
expect editor.page is 2
expect toast is Added 12 pages: they start at page 3.
expect editor.pages.undo is 1
expect editor.pages.undo.kind is add
expect editor.tile.3 is Page 3 (new)
expect editor.tile.14 is Page 14 (new)
expect editor.tile.15 is two
shot 03-added
# Undo: the twelve go, the open page still open
tap panel pages.undo-add
wait 300
expect editor.page_ids is 1,6,2,3,4,5
expect editor.page is 2
expect editor.modal is pages
expect toast is Undone: 12 new pages taken back.
expect editor.pages.undo is 0
# Cancel adds none
touch down 394 2395
wait 700
touch up 394 2395
wait 100
tap panel pages.key.3
tap panel pages.key.cancel
wait 200
expect editor.pages.adding is 0
expect editor.page_ids is 1,6,2,3,4,5
expect editor.modal is pages
# Tapped with Shift held on a keyboard (its held modifiers ride on each pen and touch event, input_evdev.rs):
# the pad at once, at 1, and a keyboard's keys type in it (the first digit replacing the 1). Shift alone is no key: nothing typed.
key 42 0
wait 100
expect editor.pages.adding is 0
tap panel pages.new
wait 200
keyup 42 0
expect editor.pages.adding is 1
expect editor.pages.text is 1
expect editor.page_ids is 1,6,2,3,4,5
key 3 0
key 79 0
wait 100
expect editor.pages.text is 21
key 14 0
key 13 2
key 13 2
wait 100
expect editor.pages.text is 4
# (Up and Down do nothing to it since 2.0.0-beta.2; - does)
key 108 0
key 103 0
wait 100
expect editor.pages.text is 4
key 12 0
wait 100
expect editor.pages.text is 3
key 28 0
wait 300
expect editor.pages.adding is 0
expect editor.page_ids is 1,6,19,20,21,2,3,4,5
expect toast is Added 3 pages: they start at page 3.
# A page of the batch opened and written on: no longer blank, Undo goes
tap panel pages.page.4
tap panel pages.page.4
wait 300
expect editor.modal is none
expect editor.page is 4
pen line 300 1200 900 1200 20 0.6
wait 100
pentap panel pages
wait 300
tap panel pages.page.2
tap panel pages.page.2
wait 300
expect editor.page is 2
tap panel pages
wait 300
expect editor.pages.undo is 0
# Esc on the pad closes it with nothing added
touch down 394 2395
wait 700
touch up 394 2395
wait 100
key 5 0
key 1 0
wait 200
expect editor.pages.adding is 0
expect editor.page_ids is 1,6,19,20,21,2,3,4,5
quit
