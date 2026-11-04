# rust-only
# generate: tests/move_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# New page's batch, a large one (review of b3-padd, 2026-11-03): from 25 pages the writing takes a while on a
# Kindle (each page file synced, 15 to 25 ms), so the pad goes first, in one refresh, before a file is written
# (with no toast since 2.0.0-beta.2: a batch is quick, the owner ruled). A contact made while the pages were being
# written (read meanwhile, aimed at the pad) is let go, down to its lift: here a pen tap on + queued with the
# Enter that started the batch adds no page. The next tap after it works as ever. Undo of the batch is offered
# only while none of its pages is named or linked (as Undo itself requires).
wait 300
clock 1700000000
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5
tap panel pages
waitbg
wait 200
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
key 4 0
key 11 0
wait 100
expect editor.pages.text is 30
expect display.presents not -1
# Enter, and a pen tap on + read while the pages are written (the same batch of input)
key 28 0
pen down 394 2395
pen up 394 2395
wait 300
expect editor.pages.adding is 0
expect editor.modal is pages
expect editor.pages is 35
expect toast is
# (the first: the pad gone, before a file is written; then the grid with the pages, as
# any batch draws it; the previews' tiles may follow, as they come)
expect display.presents has ui 0 0 1860 2480, ui 0 0 1860 2480
shot 01-added
# Undo is offered while the pages are unused. One of them named (held page 2, Rename, "a"): in use, so
# Undo is no longer offered (it would only refuse with a toast)
expect editor.pages.undo.kind is add
expect editor.pages.at.page.2 is 930 551
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
tap panel pages.rename
wait 200
tap panel pages.key.a
tap panel pages.key.done
wait 200
expect editor.tile.2 is a
expect editor.pages.undo is 0
expect editor.pages.undo.kind is
# The next tap is no longer quiet: one page at once, the view staying
tap panel pages.new
wait 300
expect editor.modal is pages
expect editor.pages is 36
expect toast is
quit
