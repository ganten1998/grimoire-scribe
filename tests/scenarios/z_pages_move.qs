# rust-only
# generate: tests/move_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# Move to... (a held page's card, and the bottom row with pages ticked): Go to page's number pad says
# the page they go before, or one past the last for the end; the pages stand together in their order,
# the open page stays open where it went, and only the order changes. Copy to notebook...: the notebook
# picker without the open notebook, a page there (the copies go before it) or At the end; the copies
# take their names and links (a link to a page copied with it points at its copy there, one to the
# destination notebook's own page becomes a page link there, one to the Index is left out), and their
# areas scale to the other notebook's page size (B is a Scribe 3's).
wait 300
tap library folder.Void
tap library notebook.A
expect editor.page_ids is 1,2,3,4,5
tap panel pages
waitbg
wait 200
expect editor.modal is pages
shot 01-bottom-row
# Page 2 held (tile 2's centre 930 551): its card has Move to... and Copy to notebook...
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
shot 02-card
tap panel pages.move-to
wait 100
expect editor.keyboard is Move before page
expect toast has Enter a page from 1 to 5, or 6 for the end.
shot 03-move-pad
tap panel pages.key.5
tap panel pages.key.done
wait 200
expect editor.keyboard is
expect editor.modal is pages
expect editor.page_ids is 1,3,4,2,5
expect editor.tile.4 is two
expect toast is
expect editor.page is 1
expect editor.ticked is 0
shot 04-moved
# Pages 1 and 5 ticked, the bottom row's Move to...: a number past the end is refused, 6 is the end.
tap panel pages.multi
tap panel pages.page.1
tap panel pages.page.5
expect editor.ticked is 2
tap panel pages.move
wait 100
expect editor.keyboard is Move before page
tap panel pages.key.9
tap panel pages.key.done
wait 100
expect editor.keyboard is Move before page
expect toast has Enter a page from 1 to 5, or 6 for the end.
tap panel pages.key.del
tap panel pages.key.6
tap panel pages.key.done
wait 200
expect editor.page_ids is 3,4,2,1,5
expect editor.page is 4
expect editor.tile.4 is one
expect toast is
# Cancel on the pad moves nothing.
tap panel pages.page.1
tap panel pages.move
wait 100
tap panel pages.key.cancel
wait 100
expect editor.keyboard is
expect editor.page_ids is 3,4,2,1,5
# Copy to notebook...: pages three and one (tiles 1, ticked, and 4) at the end of B.
tap panel pages.page.4
expect editor.ticked is 2
tap panel pages.copy-to-notebook
wait 200
expect editor.destination is 1
expect editor.destination.copying is 1
expect editor.destination.folder is Void
shot 05-copy-picker
tap panel elsewhere.book.B
waitbg
wait 300
expect editor.destination.notebook is B
shot 06-copy-pages
tap panel elsewhere.end
wait 200
expect editor.destination is 0
expect editor.modal is pages
expect toast is
expect editor.page_ids is 3,4,2,1,5
expect editor.ticked is 0
shot 07-copied
# Held page 2 (tile 2: four) copied before B's first page.
touch down 930 551
wait 700
touch up 930 551
tap panel pages.copy-to
wait 200
expect editor.destination.copying is 1
tap panel elsewhere.book.B
waitbg
wait 200
tap panel elsewhere.page.1
wait 200
expect toast is
tap panel pages.back
wait 200
# B now: four, B first, B second, three, one.
tap panel library
waitbg
tap library notebook.B
expect editor.notebook_id is B
expect editor.page_ids is 5,1,2,3,4
tap panel pages
waitbg
wait 200
expect editor.tile.1 is four
expect editor.tile.4 is three
expect editor.tile.5 is one
shot 08-destination
tap panel pages.page.4
tap panel pages.page.4
wait 200
expect editor.page is 4
expect editor.links is 1 1
tap panel pages
tap panel pages.page.5
tap panel pages.page.5
wait 200
expect editor.page is 5
expect editor.links is 1 1
quit
