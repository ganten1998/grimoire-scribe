# settings: status_mode=1
# Links drawn on pages: the toolbar Links action arms a dashed lasso around what links (a dot
# is too small, and the strip stays), then its page picked; the lasso is never ink. A finger's tap
# follows it, a finger held on it shows the long-press card, which changes where it goes (another page, the
# current notebook via the destination browser) or deletes it; Back
# while picking makes no link; Undo takes a link just made away and Redo brings it back; the links stay
# through closing and opening the notebook.
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index .
# copy: tests/fixtures/scen-tpl-nobg/notebooks/20261020-000000-0005 notebooks/link-origin
# settings: mirror_pdf=0
# rust-only
wait 100
tap library notebook.link-origin
wait 200
pentap panel next
pentap panel next
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
pen line 300 600 900 620 30 0.5 0.9
expect editor.strokes is 1
# The toolbar Links action replaces the retired Index footer.
pentap panel links
shot 01-toolbar-add-a-link
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
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.2370 is linkto.pages
tap panel linkto.pages
expect editor.modal is pages
expect editor.picking is new
expect editor.pages_tab is pages
expect toast is Where does the link go? Tap its page.
shot 02-pick-new
tap panel pages.page.3
expect editor.modal is none
expect editor.page is 1
expect editor.links is 1 1
expect editor.link.1 is p3
# (the first link this data root makes: the one-time tip instead of where it goes, hints bit 1)
expect toast is Linked. Tap it with a finger to go there; hold it for more.
# A finger's tap on it turns to page 3; one off it does nothing
tap 600 1500
expect editor.page is 1
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# A finger held on it: its border and the long-press card over the page (PHASE2 8), to change where it goes
pen leave
wait 1100
touch down 600 1100
wait 800
expect editor.link_card is 1
expect editor.link_card.title is Page 3
expect editor.link_card.rows is rename retarget redraw delete
touch up 600 1100
expect editor.modal is card
expect editor.page is 1
shot 03-card
# Change where it goes...: Link to..., without Choose from the pages...; its Search finds page 2
expect editor.ui_at.679.1558 is card.retarget
tap 679 1558
wait 100
expect editor.linkto is pick
expect editor.ui_at.480.2370 is linkto.search
tap panel linkto.search
wait 100
expect editor.find is pick
tap 310 1725
expect editor.find.1 is Link to page 2
tap 1643 2365
wait 100
expect editor.modal is none
expect toast is Linked to page 2.
expect editor.link.1 is p2
expect editor.page is 1
tap 600 1100
expect editor.page is 2
pentap panel previous
expect editor.page is 1
# Retarget through Notebooks to a page in this same notebook: a local p3 target.
pen leave
wait 1100
touch down 600 1100
wait 800
touch up 600 1100
tap 679 1558
wait 100
tap panel linkto.elsewhere
expect editor.destination is 1
tap panel elsewhere.book.link-origin
expect editor.destination.notebook is link-origin
tap panel elsewhere.page.3
expect editor.destination is 0
expect editor.link.1 is p3
expect toast is Linked to page 3.
tap 600 1100
expect editor.page is 3
expect editor.modal is none
pentap panel previous
pentap panel previous
expect editor.page is 1
# ... deleted (the lasso left no ink: only the first stroke is on the page)
pen leave
wait 1100
touch down 600 1100
wait 800
touch up 600 1100
expect editor.link_card.rows is rename retarget redraw delete
expect editor.ui_at.679.1818 is card.delete
tap 679 1818
expect editor.links is 0 0
expect editor.strokes is 1
expect toast is Link deleted. Undo brings it back.
tap 600 1100
expect editor.modal is none
expect editor.page is 1
# Back while picking: no link made, and no ink was ever drawn for it
pentap panel links
pen line 300 1400 900 1500 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.679 is linkto.pages
tap panel linkto.pages
expect editor.picking is new
expect editor.strokes is 1
tap panel pages.back
expect editor.linkto is pick
expect editor.ui_at.1530.679 is linkto.cancel
tap panel linkto.cancel
expect toast is No link made.
expect editor.strokes is 1
expect editor.links is 0 0
expect editor.live_matches_render is 1
# Making a link is an undo step: Undo takes the link away, Redo brings it back
pentap panel links
pen line 300 1400 900 1500 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.679 is linkto.pages
tap panel linkto.pages
expect editor.modal is pages
expect editor.pages_tab is pages
tap panel pages.page.3
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
