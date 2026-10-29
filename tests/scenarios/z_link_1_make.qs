# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_CHECK_FIT=1
# Making a link without ink (PHASE2 6): the strip, the dashed lasso, Link to... docked away from the area, a
# chip that picks, undo and redo, the top dock, Cancel, a zoomed view, Choose from the pages... and sideways.
# The root's page 1 has three links (to page 3 twice and to page 2), page 3 is Árbol and page 5 Arena.
wait 300
tap 200 400
wait 300
expect editor.page is 1
expect editor.links is 3 3
expect editor.strokes is 0
key 38 0
expect editor.strip is new
expect editor.link_mode is 1
expect editor.linkto is 0
shot 01-strip
# A dot is too small: the strip stays, and no ink
pentap 600 1700
expect toast is Too small for a link: draw around what links.
expect editor.strip is new
expect editor.strokes is 0
expect editor.live_matches_render is 1
# The lasso: no ink, no pen mode; Link to... at the bottom (the area is high on the page)
pen line 1000 600 1500 800 30 0.6 0.6
expect editor.strokes is 0
expect display.pen_mode is 0
expect display.last_fast not ink
expect editor.strip is 0
expect editor.linkto is pick
expect editor.modal is linkto
expect editor.linkto.dock is bottom
expect editor.link_area is 1000 600 500 200
expect editor.linkto.rect is 20 1864 1820 596
expect editor.linkto.chips is 4
expect editor.linkto.1 is new
expect editor.linkto.2 is index
expect editor.linkto.3 is p3
expect editor.linkto.4 is p5
expect editor.linkto.5 is
expect editor.linkto.screen is 1/1
expect editor.ui_at.479.2078 is linkto.chip.1
expect editor.ui_at.1381.2078 is linkto.chip.2
expect editor.ui_at.330.2370 is linkto.search
expect editor.ui_at.930.2370 is linkto.pages
expect editor.ui_at.1530.2370 is linkto.cancel
expect editor.ui_at.900.900 is
expect editor.layers is ok
shot 02-linkto
# The Árbol chip
expect editor.ui_at.479.2218 is linkto.chip.3
tap 479 2218
wait 100
expect toast is Linked to Árbol.
expect editor.links is 4 4
expect editor.undo_kind is link
expect editor.strip is 0
expect editor.linkto is 0
expect editor.modal is none
expect editor.recent is 3
expect editor.strokes is 0
# It follows with a finger, and Back comes back
pen leave
wait 1100
tap 1250 700
expect editor.page is 3
key 48 0
expect editor.page is 1
# One undo step: Undo takes it away, Redo brings it back
pentap panel undo
wait 300
expect editor.links is 3 3
expect toast is Link change undone.
pentap panel redo
wait 300
expect editor.links is 4 4
expect toast is Link change redone.
# A lasso low on the page: Link to... docks at the top, below the clock corner; the recent target is offered
key 38 0
pen line 300 2000 900 2150 30 0.6 0.6
expect editor.linkto.dock is top
expect editor.linkto.rect is 20 173 1820 736
expect editor.linkto.chips is 5
expect editor.linkto.3 is recent p3
expect editor.linkto.4 is p3
shot 03-linkto-top
expect editor.ui_at.1530.819 is linkto.cancel
tap 1530 819
wait 100
expect toast is No link made.
expect editor.linkto is 0
expect editor.link_area is
expect editor.links is 4 4
# Choose from the pages...: the Pages view picks, and closes on the page
key 38 0
pen line 1000 1000 1500 1200 30 0.6 0.6
expect editor.linkto.dock is bottom
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
expect editor.modal is pages
expect editor.picking is new
expect toast is Where does the link go? Tap its page.
tap 450 95
expect editor.pages_tab is pages
tap 703 340
wait 100
expect toast is Linked to page 2.
expect editor.modal is none
expect editor.links is 5 5
expect editor.recent is 2 3
# Zoomed in: the lasso's area is the page rect of the screen line, through the view
pinch 930 1240 300 750 6
wait 300
expect editor.zoomed is 1
key 38 0
pen line 600 600 900 800 30 0.6 0.6
expect editor.linkto is pick
# (pinched to 2x about 930 1240: page = 930 + (600 - 930) / 2, 1240 + (600 - 1240) / 2)
expect editor.link_area is 765 920 150 100
key 38 0
expect toast is No link made.
expect editor.links is 5 5
# Sideways: the strip, a lasso and Link to...
rotate 1
wait 300
key 38 0
expect editor.strip is new
shot 04-strip-sideways
pen line 1000 500 1400 700 30 0.6 0.6
expect editor.linkto is pick
expect editor.linkto.dock is bottom
expect editor.layers is ok
shot 05-linkto-sideways
key 38 0
expect toast is No link made.
expect editor.linkto is 0
rotate 0
wait 300
expect editor.links is 5 5
expect editor.strokes is 0
pentap panel library
waitbg
