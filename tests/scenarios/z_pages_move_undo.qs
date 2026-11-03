# rust-only
# generate: tests/move_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# Move to... is one undo step (owner, 2026-11-02). Right after a move, with nothing ticked, the bottom row's
# Undo (in Move to...'s place) puts the pages back in the order they had, the open page still open. It stays
# offered while the view closes and opens again and through a rename (names go by page), until a page is
# added, deleted or copied, another move replaces it, or the notebook closes. The editor's own Undo keeps to
# the open page's history: it never undoes a move.
wait 300
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5
tap panel pages
waitbg
wait 200
expect editor.pages.undo is 0
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
expect editor.pages.undo is 1
shot 01-undo-offered
tap panel pages.undo-move
wait 200
expect editor.page_ids is 1,2,3,4,5
expect editor.pages.undo is 0
expect toast is Move undone: the pages are back in their order.
expect editor.page is 1
expect editor.modal is pages
shot 02-undone
# Pages 1 (the open one) and 5 ticked, before page 3: ticks hide Undo, Move to... is in its place again
tap panel pages.multi
tap panel pages.page.1
tap panel pages.page.5
expect editor.ticked is 2
tap panel pages.move
wait 100
tap panel pages.key.3
tap panel pages.key.done
wait 200
expect editor.page_ids is 2,1,5,3,4
expect editor.page is 2
expect editor.pages.undo is 1
tap panel pages.page.4
expect editor.ticked is 1
expect editor.pages.undo is 0
wait 500
tap panel pages.page.4
expect editor.ticked is 0
expect editor.pages.undo is 1
# Closed and opened again, and a page renamed (held page 4, "three", to "threea"): still offered
tap panel pages.back
wait 200
expect editor.modal is none
tap panel pages
wait 200
expect editor.pages.undo is 1
touch down 318 1249
wait 700
touch up 318 1249
expect editor.pages.context is 4
tap panel pages.rename
wait 200
tap panel pages.key.a
tap panel pages.key.done
wait 200
expect editor.tile.4 is threea
expect editor.pages.undo is 1
tap panel pages.undo-move
wait 200
expect editor.page_ids is 1,2,3,4,5
expect editor.page is 1
expect editor.tile.3 is threea
expect editor.pages.undo is 0
# A move, then New page: the move is no longer the newest change, its Undo goes (New page's own, a batch of
# one, takes its place; the view stays)
touch down 930 551
wait 700
touch up 930 551
tap panel pages.move-to
wait 100
tap panel pages.key.1
tap panel pages.key.done
wait 200
expect editor.page_ids is 2,1,3,4,5
expect editor.pages.undo is 1
tap panel pages.new
wait 300
expect editor.modal is pages
expect editor.page_ids is 2,1,6,3,4,5
expect editor.pages.undo is 1
expect editor.pages.undo.kind is add
# A move, then the editor's Undo on the page: the move stays, and so does its Undo
touch down 930 551
wait 700
touch up 930 551
tap panel pages.move-to
wait 100
tap panel pages.key.done
wait 200
expect editor.page_ids is 2,6,3,4,5,1
tap panel pages.back
wait 200
pen line 300 1200 900 1200 20 0.6
wait 100
expect editor.strokes is 1
pentap panel undo
wait 100
expect editor.strokes is 0
expect editor.page_ids is 2,6,3,4,5,1
tap panel pages
wait 200
expect editor.pages.undo is 1
# The notebook closed: the move can no longer be undone
tap panel pages.back
wait 200
tap panel library
waitbg
tap library notebook.A
tap panel pages
wait 200
expect editor.page_ids is 2,6,3,4,5,1
expect editor.pages.undo is 0
quit
