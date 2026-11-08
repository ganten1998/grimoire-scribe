# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=pencil,fineliner
# rust-only
# Quick sizes (2.1, lane L1): a pen button on the toolbar held 600 ms (HOLD_MS) opens a card beside it with four
# widths in millimetres and a Pen feel chip. The hold also makes that pen the pen in use, as its tap would; the lift
# after a hold never acts as a tap (the eraser does not go back to the pen before it); a tap on a chip sets the
# tool's two widths and closes the card. A press shorter than the hold is the tap it always was, a hold that
# moves first opens no card, and a hold on the grid (its pen pinned) opens the picker at the lift, as its tap.
# Sizes are settings: nothing on the page, no history step.
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 3600
tap 965 95
wait 200
tap library paper.1
tap 1564 2254
wait 400
expect screen is editor
expect editor.pins is pencil,fineliner
expect editor.tool_button is grid
expect editor.at.tool:fineliner is 1768 574
expect editor.at.eraser is 1768 682
expect editor.quicksize is 0
expect editor.modal is none
expect editor.tool is pencil
# A 300 ms press on the pinned fineliner is its tap: the pen in use, no card.
touch down 1768 574
wait 300
touch up 1768 574
wait 200
expect editor.tool is fineliner
expect editor.quicksize is 0
expect editor.modal is none
# Held 700 ms on the pinned pencil but moved 40 px first: no card (the lift is its tap, as before).
touch down 1768 466
touch move 1768 506
wait 700
expect editor.quicksize is 0
touch up 1768 506
wait 200
expect editor.quicksize is 0
expect editor.tool is pencil
# The eraser held: the pen in use at once, the pencil remembered, the card beside the toolbar on one refresh.
expect display.presents not -1
touch down 1768 682
wait 300
expect editor.quicksize is 0
expect editor.owner is panel finger
wait 400
expect editor.quicksize is eraser
expect editor.modal is quick-size
expect editor.owner is quick-size finger
expect editor.tool is eraser
expect editor.before_eraser is pencil
expect editor.quicksize.current is 2
expect editor.quicksize.chip.0 is 2 mm
expect editor.quicksize.chip.3 is 12 mm
expect editor.quicksize.rect is 774 632 920 416
# (the button's press look, then the card with the whole toolbar, whose marks moved with the pen, in one Ui)
expect display.presents is fast 1718 632 100 100, ui 774 220 1058 1950
shot 01-eraser-card
# The lift after the hold acts as nothing: the card stays, the eraser is not put back to the pencil.
touch up 1768 682
wait 200
expect editor.quicksize is eraser
expect editor.tool is eraser
expect editor.owner is none
# The 12 mm chip: the eraser's two widths, the card closed, one refresh of the card, no history step.
expect editor.undo_kind is
tap panel quicksize.3
wait 200
expect settings.eraser.size_max_mm is 12.0000
expect settings.eraser.size_min_mm is 4.2900
expect editor.quicksize is 0
expect editor.modal is none
expect editor.tool is eraser
expect editor.strokes is 0
expect editor.undo_kind is
# (the chip's press look, then one Ui of the card)
expect display.presents is fast 1326 732 160 170, ui 774 632 920 416
# A hold on the grid opens the picker at the lift, as its tap does (no card).
touch down 1768 358
wait 700
expect editor.quicksize is 0
expect editor.modal is none
touch up 1768 358
wait 200
expect editor.modal is tools
expect editor.quicksize is 0
