# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=fineliner
# copy: tests/fixtures/size-keys .
# rust-only
# The size keys from a shortcuts.ini that binds them (quick-size has no starter key: this file gives it Q). quick-size
# opens the sizes card of the pen in use beside its button on the toolbar, as a hold on that button does, and its own
# key closes it again, as a menu's does; with the toolbar folded the card opens beside the folded box. A size key with
# the card up is a pick: the card closes, the pen takes the next chip and the toast says the size, with no history
# step. With Select mode on (the lasso has no size) neither key does anything.
wait 3600
tap library add
tap library add.new-notebook
wait 200
tap library paper.1
tap 1564 2254
wait 400
expect screen is editor
expect editor.pins is fineliner
tap panel tool:fineliner
wait 200
expect editor.tool is fineliner
expect editor.at.tool:fineliner is 1768 466
expect settings.fineliner.size_max_mm is 0.2500
# Q: the fineliner's card beside its pin, its 0.25 mm chip lit, no contact owning it
key 16 0
wait 200
expect editor.quicksize is fineliner
expect editor.modal is quick-size
expect editor.owner is none
expect editor.quicksize.current is 0
expect editor.quicksize.rect is 782 416 912 294
shot 01-quick-key-card
# Q again: closed
key 16 0
wait 200
expect editor.quicksize is 0
expect editor.modal is none
# Q, then Shift+]: a pick (the card closes), the next chip, the toast
key 16 0
wait 200
expect editor.quicksize is fineliner
key 27 2
wait 200
expect editor.quicksize is 0
expect settings.fineliner.size_max_mm is 0.3500
expect toast is Size: 0.35 mm
expect editor.undo_kind is
# The eraser in use: its card beside the eraser's button; Esc closes it
key 18 0
wait 200
expect editor.tool is eraser
key 16 0
wait 200
expect editor.quicksize is eraser
expect editor.quicksize.current is 3
expect editor.quicksize.rect is 782 524 912 416
key 1 0
wait 200
expect editor.quicksize is 0
# The toolbar folded: the card beside the folded box
key 48 0
wait 300
expect editor.panel_rect is 1712 220 120 120
key 16 0
wait 200
expect editor.quicksize is eraser
expect editor.quicksize.rect is 776 220 912 416
shot 02-quick-key-folded
key 16 0
wait 200
expect editor.quicksize is 0
key 48 0
wait 300
# Select mode on: neither key does anything (the earlier toasts long gone)
key 18 0
wait 200
expect editor.tool is fineliner
key 31 0
wait 1500
expect editor.select is armed
expect toast is
key 16 0
wait 200
key 27 2
wait 200
expect editor.quicksize is 0
expect toast is
expect settings.fineliner.size_max_mm is 0.3500
