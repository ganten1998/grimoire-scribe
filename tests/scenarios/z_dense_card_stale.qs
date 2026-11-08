# rust-only
# generate: tests/previews_fixture.py ghosts
# A Library card that is stale as the notebook opens (the ghost notebook's: the old ink of a page 1 that was cleared,
# made by 2.0.0 or 2.0.1, nothing noting what it was drawn from) stays stale if the close trusts the card of the
# open for page 1's as it is: the close from another page left it alone. It is drawn again from page 1 as it is, once
# (its Pages preview, made from the same old picture, is no more trusted than the card).
wait 300
tap library notebook.ghost
expect editor.page is 2
expect editor.card_matches_render is 0
pen line 400 900 1000 910 40 0.6 0.6
pen leave
wait 400
tap panel library
wait 600
expect screen is library
tap library notebook.ghost
expect editor.page is 2
expect editor.card_matches_render is 1
# and a close with no change on any page leaves the card as it is, however often (nothing is drawn again)
tap panel library
wait 600
tap library notebook.ghost
expect editor.card_matches_render is 1
quit
