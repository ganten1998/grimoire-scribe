# rust-only
# generate: tests/notebook_backlinks_fixture.py many
# env: GRIMOIRE_CHECK_FIT=1
# The Links bar up before the backlinks cache's first check is through (120 more notebooks to read first, one
# a tick): its Linked from rows come in by themselves once the check learns them, the bar built and drawn
# again (2.0.0-beta.2). A long press on a Linked from row is its tap: it has no links of this page for a card.
wait 300
tap library folder.Other
tap library notebook.C
expect editor.notebook_id is C
pentap panel next
expect editor.page is 2
key 38 0
expect editor.bar is open
expect editor.bar.incoming is 0
shot 01-not-known-yet
wait 9000
expect editor.bar is open
expect editor.bar.incoming is 2
expect editor.bar.from.1 is n:A:p1
expect editor.bar.from.2 is n:B:p3
shot 02-learned
# Held past the long press (chip 1, Origin / Page 1): no card, and the lift follows the row.
touch down 477 2217
wait 800
expect editor.link_card is 0
touch up 477 2217
expect editor.notebook_id is A
expect editor.page is 1
expect editor.notebook_back is 1 C
quit
