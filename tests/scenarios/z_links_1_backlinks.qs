# rust-only
# generate: tests/notebook_backlinks_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
# Links between notebooks (2.0.0-beta.2): C's page 2 is linked from A's page 1 and B's page 3. Its Links
# bar lists both under Linked from (the backlinks cache, built on idle ticks); a row follows the link back,
# and Back returns to C's page 2 at the zoom it was left at.
wait 300
tap library folder.Other
tap library notebook.C
expect editor.notebook_id is C
pentap panel next
expect editor.page is 2
pen leave
wait 1100
key 38 0
expect editor.bar is open
expect editor.bar.entries is 0
expect editor.bar.incoming is 2
expect editor.bar.from.1 is n:A:p1
expect editor.bar.from.2 is n:B:p3
shot 01-linked-from
key 45 0
expect editor.bar is 0
pinch 930 1200 300 900 6
expect editor.zoomed is 1
expect editor.view is 2.25 517 667
key 38 0
expect editor.bar is open
tap panel bar.chip.2
expect editor.notebook_id is B
expect editor.page is 3
expect editor.zoom is 1.00
expect editor.notebook_back is 1 C
shot 02-followed-to-b
# B's page 3 lists C's page 2 as where its link goes; Back goes to C at the zoom it had.
key 48 0
expect editor.notebook_id is C
expect editor.page is 2
expect editor.view is 2.25 517 667
expect editor.notebook_back is 0
shot 03-back-zoomed
# Page 2's card in the Pages view says how many pages link to it.
tap panel pages
waitbg
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
expect editor.pages.context.from is 2
shot 04-page-card
tap panel pages.cancel
expect editor.pages.context is 0
# The Graph: C's page 2 with A's page 1 and B's page 3, which link to it (every notebook's: there is no
# Notebook choice); A's page 1, tapped twice, opens there.
tap panel pages.graph
expect editor.map is graph
# (the Graph is every notebook's: there is no Notebook choice, the owner's ruling of 2026-11-03)
expect editor.map.scope is all
expect editor.map.visible.ids is A:1 A:2 B:3 C:2
expect editor.map.edges is 3
shot 05-graph-across
tap map-node A:1
expect editor.map.selected.id is A
tap map-node A:1
expect editor.notebook_id is A
expect editor.page is 1
quit
