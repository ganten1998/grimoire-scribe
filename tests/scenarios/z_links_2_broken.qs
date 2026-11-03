# rust-only
# generate: tests/notebook_backlinks_fixture.py broken
# env: GRIMOIRE_CHECK_FIT=1
# Broken links (2.0.0-beta.2): A's page 1 links to C's page 2, which was deleted, and A's page 2 to a
# notebook that is gone. Each shows as such (a dashed chip, Page gone / Notebook gone) and a tap opens its
# card, to change where it goes or delete it, instead of a toast and nothing.
wait 300
tap library folder.Void
tap library notebook.A
expect editor.notebook_id is A
expect editor.links is 2 2
pen leave
wait 1100
key 38 0
expect editor.bar is open
expect editor.bar.entries is 2
expect editor.bar.1 is p2
expect editor.bar.2 is gone
shot 01-gone-chip
tap panel bar.chip.2
expect editor.link_card.title is Page gone · Destination C
expect editor.link_card.rows is retarget redraw delete
expect editor.link_card.links is 1
shot 02-gone-card
key 45 0
key 45 0
expect editor.bar is 0
# A tap on the link on the page opens the same card; Delete link takes it away, Undo brings it back.
tap 500 1000
expect editor.link_card.title is Page gone · Destination C
shot 03-gone-card-from-page
expect editor.ui_at.1125.1342 is card.delete
tap 1125 1342
expect editor.links is 1 1
expect editor.link_card.title is
tap panel undo
expect editor.links is 2 2
# Page 2 links to a notebook that is gone.
pentap panel next
expect editor.page is 2
pen leave
wait 1100
tap 500 600
expect editor.link_card.title is Notebook gone
shot 04-notebook-gone
key 45 0
key 38 0
expect editor.bar.1 is gone
key 45 0
# The Graph: both broken links are no nodes, and the panel counts them; the List's row of one opens the link's
# card on its page.
tap panel pages
tap panel pages.graph
# (the Graph is every notebook's: there is no Notebook choice, the owner's ruling of 2026-11-03)
expect editor.map.scope is all
expect editor.map.broken is 2
expect editor.map.nodes is 2
shot 05-graph-broken
tap panel map.list
expect editor.map.list is 1
# (row 1 is page 1's link to its page 2; row 2 its link to C's deleted page)
tap panel map.link.2
expect editor.page is 1
expect editor.link_card.title is Page gone · Destination C
quit
