# settings: status_mode=1
# Links drawn on pages: Add a link in the Index, then the dashed lasso drawn around what links (a dot
# is too small, and the strip stays), then its page picked; the lasso is never ink. A finger's tap
# follows it, a finger held on it changes where it goes (another page, the Index) or removes it; Back
# while picking makes no link; Undo takes a link just made away and Redo brings it back; the links stay
# through closing and opening the notebook.
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index .
# rust-only
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
pen line 300 600 900 620 30 0.5 0.9
expect editor.strokes is 1
# Add a link, from the Index tab of the Pages view
pentap panel pages
expect editor.modal is pages
tap 850 95
expect editor.pages_tab is index
shot 01-index-add-a-link
tap 1410 2395
expect editor.modal is none
expect editor.link_mode is 1
expect editor.strip is new
pentap 600 1200
expect toast is Too small for a link: draw around what links.
expect editor.strokes is 1
expect editor.link_mode is 1
expect editor.live_matches_render is 1
pen line 300 1000 900 1200 30 0.6 0.6
expect editor.strokes is 1
expect editor.link_mode is 0
expect editor.modal is pages
expect editor.picking is new
expect editor.pages_tab is pages
expect toast is Where does the link go? Tap its page.
shot 02-pick-new
tap 1155 340
expect editor.modal is none
expect editor.page is 1
expect editor.links is 1 1
expect editor.link.1 is p3
expect toast is Linked to page 3.
# A finger's tap on it turns to page 3; one off it does nothing
tap 600 1500
expect editor.page is 1
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# A finger held on it: the Pages view, to change where it goes
touch down 600 1100
wait 800
expect editor.picking is had
expect toast is The link goes to page 3. Tap another to change it.
touch up 600 1100
expect editor.modal is pages
shot 03-pick-had
tap 703 340
expect editor.modal is none
expect editor.link.1 is p2
expect editor.page is 1
tap 600 1100
expect editor.page is 2
pentap panel previous
expect editor.page is 1
# ... to the Index
touch down 600 1100
wait 800
touch up 600 1100
tap 1250 95
expect editor.link.1 is index
expect toast is The link goes to the Index.
tap 600 1100
expect editor.modal is pages
expect editor.pages_tab is index
tap 125 95
expect editor.modal is none
# ... removed (the lasso left no ink: only the first stroke is on the page)
touch down 600 1100
wait 800
touch up 600 1100
tap 1630 95
expect editor.links is 0 0
expect editor.strokes is 1
expect toast is Link removed. Its ink stays on the page.
tap 600 1100
expect editor.modal is none
expect editor.page is 1
# Back while picking: no link made, and no ink was ever drawn for it
pentap panel pages
expect editor.pages_tab is pages
tap 850 95
tap 1410 2395
pen line 300 1400 900 1500 30 0.6 0.6
expect editor.picking is new
expect editor.strokes is 1
tap 125 95
expect toast is No link made.
expect editor.strokes is 1
expect editor.links is 0 0
expect editor.live_matches_render is 1
# Making a link is an undo step: Undo takes the link away, Redo brings it back
pentap panel pages
tap 850 95
tap 1410 2395
pen line 300 1400 900 1500 30 0.6 0.6
expect editor.modal is pages
expect editor.pages_tab is pages
tap 1155 340
expect editor.links is 1 1
expect editor.link.1 is p3
pentap panel undo
wait 300
expect editor.links is 0 0
expect editor.redo_kind is link
expect toast is Link change undone.
tap 600 1450
expect editor.page is 1
pentap panel redo
wait 300
expect editor.links is 1 1
expect toast is Link change redone.
expect editor.undo_kind is link
# The next ink never joins a link's box
pen line 1200 1800 1500 1900 30 0.6 0.6
tap 1350 1850
expect editor.page is 1
# It stays through closing and opening the notebook
pentap panel library
waitbg
tap 200 400
expect editor.page is 1
expect editor.links is 1 1
tap 600 1450
expect editor.page is 3
quit
