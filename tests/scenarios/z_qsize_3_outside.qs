# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=fineliner
# rust-only
# Quick sizes, closing the card: a finger tap outside closes it at the lift and does nothing else (a tap on another
# toolbar button presses nothing); a nib down outside closes it at its down and its stroke is swallowed (no ink);
# the clock corner keeps its tap and the card stays for it; Esc closes it, one layer; a turn closes it, silently.
wait 3600
tap 965 95
wait 200
tap library paper.1
tap 1564 2254
wait 400
expect screen is editor
expect settings.status_on is 1
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
# A finger tap on another toolbar button (Smart shapes) closes the card and does nothing more.
expect settings.shapes is 1
tap panel shapes
wait 200
expect editor.quicksize is 0
expect editor.modal is none
expect settings.shapes is 1
# A finger tap on the page closes it, adds no ink and turns no page.
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
tap 400 1200
wait 200
expect editor.quicksize is 0
expect editor.strokes is 0
expect editor.page is 1
# A nib down outside closes it at its down, and the stroke it began is swallowed.
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
pen down 400 1000
expect editor.quicksize is 0
pen move 600 1100
pen move 800 1200
pen up 800 1200
wait 300
pen leave
expect editor.strokes is 0
# The clock corner keeps its tap: the clock toggles, the card stays.
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
tap 70 70
wait 200
expect settings.status_on is 0
expect editor.quicksize is fineliner
# Esc closes it.
key 1 0
wait 200
expect editor.quicksize is 0
expect editor.modal is none
# A turn closes it.
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
rotate 1
wait 500
expect editor.quicksize is 0
expect editor.modal is none
