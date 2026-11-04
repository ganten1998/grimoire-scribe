# rust-only
# generate: tests/pages_paper_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# The Pages view's Undo through the undo action, and its Redo (the owner, 2026-11-03: "Make the pages undo
# mapped to the same undo flag that the controller sees for it in pages, just keep them seperate to each other.
# ... And add the redo button."). While the Pages view is open, undo (Ctrl+Z, the controller's South) undoes its
# last move or batch and redo (Ctrl+Y, North) redoes it; the bottom row's Redo, beside Undo in Copy to
# notebook...'s place with nothing ticked, does the same. One step each; any other change to the pages drops the
# Redo. On the page, undo and redo stay the page's own: a stroke is never undone from Pages, a move never from
# the page.
wait 300
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5
expect editor.page is 2
pen line 300 1200 900 1200 20 0.6
wait 100
expect editor.strokes is 1
tap panel pages
waitbg
wait 200
expect editor.pages.redo is 0
# Page 2 (held) to the end
touch down 930 551
wait 700
touch up 930 551
tap panel pages.move-to
wait 100
tap panel pages.key.6
tap panel pages.key.done
wait 200
expect editor.page_ids is 1,3,4,5,2
expect editor.pages.undo.kind is move
# Ctrl+Z: the move undone, not the stroke; Redo offered, beside the Undo's place
key 44 1
wait 200
expect editor.modal is pages
expect editor.page_ids is 1,2,3,4,5
expect toast is
expect editor.strokes is 1
expect editor.pages.undo is 0
expect editor.pages.redo is 1
expect editor.pages.redo.kind is move
expect editor.pages.at.redo is 790 2395
shot 01-redo-offered
# Ctrl+Y: the exact order again; Undo back, Redo gone
key 21 1
wait 200
expect editor.page_ids is 1,3,4,5,2
expect toast is
expect editor.pages.redo is 0
expect editor.pages.undo.kind is move
expect editor.pages.at.redo is
expect editor.strokes is 1
# The controller's undo and redo (South, North)
key 304 0 1
wait 200
expect editor.page_ids is 1,2,3,4,5
key 307 0 1
wait 200
expect editor.page_ids is 1,3,4,5,2
# The bottom row's Redo
key 304 0 1
wait 200
tap panel pages.redo
wait 200
expect editor.page_ids is 1,3,4,5,2
expect editor.pages.redo is 0
expect editor.pages.undo is 1
shot 02-redone
# A tick hides the bottom row's Redo (Copy to notebook... is in its place again); the redo key still redoes
key 44 1
wait 200
tap panel pages.multi
tap panel pages.page.3
expect editor.ticked is 1
expect editor.pages.redo is 1
expect editor.pages.at.redo is
key 21 1
wait 200
expect editor.page_ids is 1,3,4,5,2
expect editor.ticked is 0
tap panel pages.multi
# Another change to the pages (New page) drops the Redo: redo then does nothing
key 44 1
wait 200
expect editor.page_ids is 1,2,3,4,5
expect editor.pages.redo is 1
tap panel pages.new
wait 300
expect editor.page_ids is 1,2,6,3,4,5
expect editor.pages.redo is 0
key 21 1
wait 200
expect editor.page_ids is 1,2,6,3,4,5
expect editor.strokes is 1
# The batch's Undo and Redo from the keys: the same page (id 6) back at its place
key 44 1
wait 200
expect editor.page_ids is 1,2,3,4,5
expect editor.pages.redo.kind is add
key 21 1
wait 200
expect editor.page_ids is 1,2,6,3,4,5
expect toast is
expect editor.tile.3 is Page 3 (new)
# On the page: undo and redo are the page's (the stroke), the pages stay as they are
tap panel pages.back
wait 200
expect editor.modal is none
key 44 1
wait 100
expect editor.strokes is 0
expect editor.page_ids is 1,2,6,3,4,5
key 21 1
wait 100
expect editor.strokes is 1
expect editor.page_ids is 1,2,6,3,4,5
# and back in Pages, its Undo is still the batch's
tap panel pages
wait 200
expect editor.pages.undo.kind is add
key 44 1
wait 200
expect editor.page_ids is 1,2,3,4,5
expect editor.strokes is 1
quit
