# rust-only
# generate: tests/add_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/new-pages-key .
# The toolbar's page adder (owner, 2026-11-03: "add the long press dialog to the toolbars page adder -- two birds
# one stone. swiping still works for new pages and nav so."): on the last page the next-page button is a +; held
# 600 ms (the Pages view's New page's hold) or clicked with a modifier key held, it opens the same count pad, along
# the bottom over the page. Done adds that many blank pages right after the open page (on the last page: the end),
# and the editor stays on the page it was on; a toast says how many, from which page, and that the Pages view's
# Undo takes them back (the batch is the notebook's Undo record, as the Pages view's batch). A plain tap on the +
# still adds one page and goes to it, and so does swiping past the last page (unchanged). The new-pages action
# opens the pad from a key, on any page.
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting; a slow
# runner reached the 3.5 s one inside a counted stretch)
wait 3600
clock 1700000000
tap library folder.Void
tap library notebook.S
expect editor.page_ids is 1,2
expect editor.page is 1
# Not on the last page the button is next: held, it opens no pad and turns the page at the lift, as a tap
expect editor.at.next is 1768 1330
touch down 1768 1330
wait 700
expect editor.addpad is 0
touch up 1768 1330
wait 200
expect editor.addpad is 0
expect editor.page is 2
expect editor.page_ids is 1,2
# On the last page, the + held: the count pad, over the page, at 1, Done on
expect display.presents not -1
touch down 1768 1330
wait 700
expect editor.addpad is 1
# (the + pressed, then the pad with the + let go under it, in one refresh)
expect display.presents is fast 1718 1280 100 100, ui 20 1280 1820 1180
touch up 1768 1330
wait 100
expect editor.addpad is 1
expect editor.modal is add-pad
expect editor.keyboard is Pages to add
expect editor.text is 1
expect editor.addpad.done is Add 1 page
expect editor.addpad.done.on is 1
expect editor.page is 2
shot 01-pad
# del clears the 1 and Done at 0 does nothing; 3, Done: three pages after page 2 (the end), the editor still on
# page 2
tap panel addpad.key.del
wait 100
expect editor.addpad.done.on is 0
tap panel addpad.key.done
wait 100
expect editor.addpad is 1
tap panel addpad.key.3
wait 100
expect editor.addpad.done is Add 3 pages
expect editor.addpad.done.on is 1
expect display.presents not -1
tap panel addpad.key.done
wait 300
expect editor.addpad is 0
expect editor.modal is none
expect editor.page_ids is 1,2,3,4,5
expect editor.page is 2
expect toast is Added 3 pages after this one, from page 3. Undo them in Pages.
# (the page where the pad was, with the page count and the + turned next, in one refresh)
expect display.presents is ui 20 1212 1820 1248
shot 02-added
# The Pages view: the three marked new, its bottom row's Undo takes them back; the editor still on page 2
tap panel pages
wait 300
expect editor.tile.2 is Page 2 (open)
expect editor.tile.3 is Page 3 (new)
expect editor.tile.5 is Page 5 (new)
expect editor.pages.undo.kind is add
tap panel pages.undo-add
wait 300
expect editor.page_ids is 1,2
expect toast is Undone: 3 new pages taken back.
tap panel pages.back
wait 300
expect editor.modal is none
expect editor.page is 2
# Clicked with Shift held on a keyboard: the pad at once; a keyboard's keys type in it, Enter is Done
key 42 0
tap panel next
wait 200
keyup 42 0
expect editor.addpad is 1
expect editor.page_ids is 1,2
key 3 0
wait 100
expect editor.text is 2
key 28 0
wait 300
expect editor.addpad is 0
expect editor.page_ids is 1,2,6,7
expect editor.page is 2
expect toast is Added 2 pages after this one, from page 3. Undo them in Pages.
# Not on the last page now: the button is next again, and held it turns the page (no pad)
touch down 1768 1330
wait 700
touch up 1768 1330
wait 200
expect editor.addpad is 0
expect editor.page is 3
expect editor.page_ids is 1,2,6,7
# Cancel, Esc, a finger outside and the pen outside each close the pad with nothing added (the pen's
# stroke swallowed); the new-pages key opens it on any page
key 49 0
wait 200
expect editor.addpad is 1
tap panel addpad.key.4
tap panel addpad.key.cancel
wait 200
expect editor.addpad is 0
expect editor.page_ids is 1,2,6,7
key 49 0
wait 200
key 5 0
key 1 0
wait 200
expect editor.addpad is 0
expect editor.page_ids is 1,2,6,7
key 49 0
wait 200
tap 900 600
wait 200
expect editor.addpad is 0
key 49 0
wait 200
pentap 900 600
wait 200
expect editor.addpad is 0
expect editor.strokes is 0
expect editor.page_ids is 1,2,6,7
# The new-pages key on page 3: one page after it, page 3 still open
key 49 0
wait 200
expect editor.addpad.done is Add 1 page
tap panel addpad.key.done
wait 300
expect editor.page_ids is 1,2,6,8,7
expect editor.page is 3
expect toast is Added 1 page after this one: page 4. Undo it in Pages.
# A plain tap on the + still adds one page and goes to it (unchanged), as does swiping past the last page
key 37 0
wait 200
expect editor.page is 5
tap panel next
wait 300
expect editor.page_ids is 1,2,6,8,7,9
expect editor.page is 6
expect editor.addpad is 0
wait 1100
swipe 2 1300 1200 -400 0
wait 300
expect editor.page_ids is 1,2,6,8,7,9,10
expect editor.page is 7
# A large batch: "Adding N pages..." first; a pen tap read while the pages are written (queued with the Enter
# that started it) is let go, no ink on the page
pen leave
touch down 1768 1330
wait 700
touch up 1768 1330
wait 100
expect editor.addpad is 1
key 4 0
key 11 0
wait 100
expect editor.text is 30
key 28 0
pen down 900 600
pen up 900 600
wait 300
expect editor.addpad is 0
expect editor.pages is 37
expect editor.page is 7
expect editor.strokes is 0
expect toast is Added 30 pages after this one, from page 8. Undo them in Pages.
# The next contact is no longer let go
pen line 300 1200 900 1200 20 0.6
wait 200
expect editor.strokes is 1
quit
