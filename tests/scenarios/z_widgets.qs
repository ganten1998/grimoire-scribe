# settings: status_mode=1
# rust-only
# Phase 1 widgets: the toolbar's Widgets button opens the catalog popup; a row opens a frame; the
# title's minimize button shrinks it and a tap on the title restores it; the title bar drags; the
# close button removes it.
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.widgets is 0
expect editor.widgets.open is 0
tap panel widgets
wait 300
expect editor.widgets.open is 1
shot 01-widgets-popup
tap panel widgets.demo.sticky-notes
wait 400
expect editor.widgets.open is 0
expect editor.widgets is 1 demo.sticky-notes
expect editor.widget.demo.sticky-notes.state is normal
shot 02-sticky-open
tap panel widget.demo.sticky-notes.min
wait 300
expect editor.widget.demo.sticky-notes.state is minimized
shot 03-sticky-minimized
tap panel widget.demo.sticky-notes.title
wait 300
expect editor.widget.demo.sticky-notes.state is normal
# Drag the title bar to the middle of the page. (The frame starts at the top right, beside the panel.)
drag touch 1546 60 1100 500 8
wait 300
expect editor.widget.demo.sticky-notes.state is normal
expect editor.owner is none
shot 04-sticky-dragged
# Open the calculator too, then close both.
tap panel widgets
wait 200
tap panel widgets.demo.calculator
wait 400
expect editor.widgets is 2 demo.sticky-notes demo.calculator
shot 05-two-widgets
tap panel widget.demo.calculator.close
wait 300
expect editor.widgets is 1 demo.sticky-notes
tap panel widget.demo.sticky-notes.close
wait 300
expect editor.widgets is 0
shot 06-widgets-closed
wait 1800
quit
