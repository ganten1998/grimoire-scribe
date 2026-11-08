# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=fineliner
# rust-only
# Quick sizes, the contact that opened the card slides to pick: the nib held 600 ms on the pinned fineliner, moved
# onto a chip and lifted there sets that width. A nib lifted where it was when the card opened picks nothing (the
# card may sit near the held button), and one that slid off every chip leaves the card open. The tool button,
# while it shows a pen, opens its card too.
wait 3600
tap 965 95
wait 200
tap library paper.1
tap 1564 2254
wait 400
expect screen is editor
expect editor.tool_button is pencil
expect editor.at.tool:fineliner is 1768 466
pen down 1768 466
wait 700
expect editor.quicksize is fineliner
expect editor.owner is quick-size pen
expect editor.quicksize.current is 0
expect editor.quicksize.chip.2 is 0.5 mm
expect editor.at.quicksize.2 is 1238 601
# Lifted where it was: nothing picked, the card stays.
pen up 1768 466
wait 200
expect editor.quicksize is fineliner
expect editor.tool is fineliner
expect settings.fineliner.size_max_mm is 0.2500
# A finger tap outside closes it and does nothing else.
tap 400 1200
wait 200
expect editor.quicksize is 0
pen leave
# Held again, slid onto the 0.5 chip and lifted there: picked, the card gone, nothing on the page.
pen down 1768 466
wait 700
expect editor.quicksize is fineliner
pen move 1500 520
pen move 1238 601
wait 100
pen up 1238 601
wait 200
expect settings.fineliner.size_max_mm is 0.5000
expect settings.fineliner.size_min_mm is 0.5000
expect editor.quicksize is 0
expect editor.strokes is 0
expect editor.undo_kind is
pen leave
# The tool button, while it shows a pen, opens that pen's card with a finger; slid off every chip and lifted, the
# card stays.
touch down 1768 358
wait 700
expect editor.quicksize is pencil
expect editor.owner is quick-size finger
touch move 1500 900
touch up 1500 900
wait 200
expect editor.quicksize is pencil
expect settings.pencil.size_max_mm is 0.3500
# (no Pen feel chip on the card: Pen feel stays in the tool picker and Settings)
expect editor.at.quicksize.pen-feel is
