# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
# rust-only
# The panel only ever overlaps the notebook page: every menu, sheet and bar (the Links bar, the Find sheet and its
# keyboard, the Display sheet, Paper, the tool picker, the Pages view) draws over the panel, whatever redraws it.
# editor.layers composes the screen again whole (the page, the panel, then each menu in its order) and compares it
# with what the screen held. The panel is dragged down the left of the page so that each menu covers part of it,
# and then the screen is turned: a panel repainted late (after a stroke, once a shortcut had opened a menu) used
# to lay itself over the menu.
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
# The panel down the left of the page, as low as it goes: 100 px from the left edge, to y 734 (the screen's height less its margin and the panel's 1734).
drag touch 1737 260 133 1200 6
wait 400
expect editor.panel_rect is 100 734 128 1734
expect editor.layers is ok
# The Links bar, along the bottom, over the panel's foot; closed again.
key 37 0
wait 400
expect editor.modal is links
expect editor.layers is ok
shot 01-bar-over-panel
key 37 0
wait 400
expect editor.modal is none
expect editor.layers is ok
# Find: the card and its keyboard over the panel; typing changes the card (a piece of the screen is redrawn).
key 34 0
wait 400
expect editor.modal is find
expect editor.layers is ok
tap 487 1725
wait 200
expect editor.find.rows is 1
expect editor.layers is ok
shot 02-find-over-panel
key 34 0
wait 400
expect editor.modal is none
expect editor.layers is ok
# Display, over the panel's top (the sheet is 1500 wide from x 180, and the panel 100 to 228).
tap panel display
wait 400
expect editor.modal is display
expect editor.layers is ok
shot 03-display-over-panel
tap 70 70
wait 400
expect editor.modal is none
expect editor.layers is ok
# Paper.
tap panel paper
wait 400
expect editor.modal is paper
expect editor.layers is ok
shot 04-paper-over-panel
tap 70 70
wait 400
expect editor.modal is none
expect editor.layers is ok
# The tool picker, beside the panel; a pen picked in it redraws the panel under it.
tap panel tool-picker
wait 400
expect editor.modal is tools
expect editor.layers is ok
shot 05-tools-over-panel
tap panel tool-picker
wait 400
# The Pages view covers the whole screen: no panel on it at all.
tap panel pages
tap panel map.pages
wait 500
expect editor.modal is pages
expect editor.layers is ok
shot 06-pages-view
key 45 0
wait 500
expect editor.modal is none
expect editor.layers is ok
# Dragged to the middle of the page (by its grip) and the Display sheet opened over most of it.
drag touch 133 774 833 760 6
wait 400
expect editor.modal is none
expect editor.layers is ok
tap panel display
wait 400
expect editor.layers is ok
shot 07-display-covers-the-panel
tap 70 70
wait 400
expect editor.layers is ok
# Sideways the panel and the Links bar overlap more. A stroke that just ended leaves the panel stale, and its
# repaint (at the settle, 150 ms after the pen left) used to land on whatever a shortcut had opened over the
# page meanwhile: the Links bar, the Find sheet and its keyboard.
rotate 1
wait 500
expect rotation is 1
expect editor.layers is ok
pen line 700 500 1200 700 12 0.6 0.6
key 37 0
wait 600
expect editor.modal is links
pen leave
wait 600
expect editor.layers is ok
shot 08-sideways-bar-after-a-stroke
key 37 0
wait 400
expect editor.modal is none
expect editor.layers is ok
pen line 700 500 1200 700 12 0.6 0.6
key 34 0
wait 600
expect editor.modal is find
pen leave
wait 600
expect editor.layers is ok
shot 09-sideways-find-after-a-stroke
key 34 0
wait 400
expect editor.modal is none
expect editor.layers is ok
quit
